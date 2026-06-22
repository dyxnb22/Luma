#!/usr/bin/env bash
set -euo pipefail

IDENTITY_NAME="${LUMA_LOCAL_CODESIGN_NAME:-Luma Local Development}"
KEYCHAIN="${HOME}/Library/Keychains/login.keychain-db"
TMP_DIR="$(mktemp -d)"
OPENSSL_BIN="${LUMA_OPENSSL_BIN:-/usr/bin/openssl}"
PASS="${LUMA_LOCAL_CODESIGN_P12_PASSWORD:-luma-local-dev}"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if security find-identity -v -p codesigning | grep -q "\"${IDENTITY_NAME}\""; then
  echo "Code-signing identity already installed: ${IDENTITY_NAME}"
  exit 0
fi

cat > "${TMP_DIR}/codesign.cnf" <<EOF
[ req ]
default_bits = 2048
distinguished_name = req_distinguished_name
x509_extensions = codesign_ext
prompt = no

[ req_distinguished_name ]
CN = ${IDENTITY_NAME}
O = Luma Local
OU = Development

[ codesign_ext ]
basicConstraints = critical,CA:true
keyUsage = critical,digitalSignature,keyCertSign
extendedKeyUsage = codeSigning
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
EOF

"${OPENSSL_BIN}" req \
  -newkey rsa:2048 \
  -nodes \
  -keyout "${TMP_DIR}/codesign.key" \
  -x509 \
  -days 3650 \
  -out "${TMP_DIR}/codesign.crt" \
  -config "${TMP_DIR}/codesign.cnf" \
  -sha256

"${OPENSSL_BIN}" pkcs12 \
  -export \
  -inkey "${TMP_DIR}/codesign.key" \
  -in "${TMP_DIR}/codesign.crt" \
  -name "${IDENTITY_NAME}" \
  -out "${TMP_DIR}/codesign.p12" \
  -passout "pass:${PASS}"

security import "${TMP_DIR}/codesign.p12" \
  -k "${KEYCHAIN}" \
  -P "${PASS}" \
  -A \
  -T /usr/bin/codesign \
  -T /usr/bin/security

security add-trusted-cert \
  -r trustRoot \
  -p codeSign \
  -k "${KEYCHAIN}" \
  "${TMP_DIR}/codesign.crt"

security set-key-partition-list \
  -S apple-tool:,apple:,codesign: \
  -s \
  -k "" \
  "${KEYCHAIN}" >/dev/null 2>&1 || true

echo "Installed code-signing identity: ${IDENTITY_NAME}"
security find-identity -v -p codesigning | grep "\"${IDENTITY_NAME}\"" || true
