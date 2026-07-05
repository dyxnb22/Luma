#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=scripts/release/common.sh
source "$ROOT/scripts/release/common.sh"

APP_DIR="$ROOT/build/Luma.app"
DMG_DIR="$ROOT/build/release"
DMG_PATH="$DMG_DIR/Luma.dmg"
STAGING_DIR="$DMG_DIR/dmg-staging"

cd "$ROOT"

require_developer_id() {
  local identity
  identity="$(luma_resolve_sign_identity)"
  if [[ "$identity" == "-" ]] || [[ "$identity" != *"Developer ID Application"* ]]; then
    echo "Release DMG requires LUMA_CODESIGN_IDENTITY or a Developer ID Application certificate." >&2
    echo "Set LUMA_CODESIGN_IDENTITY to your Developer ID Application identity." >&2
    exit 1
  fi
  export LUMA_CODESIGN_IDENTITY="$identity"
}

submit_for_notarization() {
  local artifact="$1"
  if [[ -n "${NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$artifact" --keychain-profile "$NOTARY_PROFILE" --wait
  else
    xcrun notarytool submit "$artifact" \
      --apple-id "$APPLE_ID" \
      --team-id "$APPLE_TEAM_ID" \
      --password "${APPLE_APP_SPECIFIC_PASSWORD:?Set APPLE_APP_SPECIFIC_PASSWORD}" \
      --wait
  fi
}

echo "Building release binary…"
luma_swift_build_release "$ROOT"

require_developer_id

echo "Assembling signed app bundle…"
luma_assemble_app "$ROOT" "$APP_DIR"

mkdir -p "$DMG_DIR"
rm -rf "$STAGING_DIR" "$DMG_PATH"
mkdir -p "$STAGING_DIR"
cp -R "$APP_DIR" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"

echo "Creating DMG…"
hdiutil create -volname "Luma" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH" >/dev/null

if [[ -n "${LUMA_SKIP_NOTARIZATION:-}" ]]; then
  echo "LUMA_SKIP_NOTARIZATION is set; skipping notarization."
else
  require_notary_credentials() {
    if [[ -z "${NOTARY_PROFILE:-}" && ( -z "${APPLE_ID:-}" || -z "${APPLE_TEAM_ID:-}" ) ]]; then
      echo "Notarization requires NOTARY_PROFILE or APPLE_ID + APPLE_TEAM_ID (+ APPLE_APP_SPECIFIC_PASSWORD)." >&2
      echo "Set LUMA_SKIP_NOTARIZATION=1 to build a signed but un-notarized DMG for local testing." >&2
      exit 1
    fi
  }

  require_notary_credentials
  echo "Submitting DMG for notarization…"
  submit_for_notarization "$DMG_PATH"
  echo "Stapling notarization ticket to DMG…"
  xcrun stapler staple "$DMG_PATH"
fi

rm -rf "$STAGING_DIR"

echo "Release artifact: $DMG_PATH"
echo "Verify with: spctl -a -vv \"$DMG_PATH\" && spctl -a -vv \"$APP_DIR\""
