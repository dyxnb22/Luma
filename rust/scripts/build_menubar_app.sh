#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUST_ROOT="$ROOT/rust"
APP="${1:-$RUST_ROOT/target/Luma Menu Bar.app}"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
BUNDLE_IDENTIFIER="com.luma.next.menubar"
SIGNING_IDENTITY="-"
if [[ -n "${CODESIGN_IDENTITY+x}" && -n "$CODESIGN_IDENTITY" ]]; then
    SIGNING_IDENTITY="$CODESIGN_IDENTITY"
fi

cd "$RUST_ROOT"
cargo build --release -p luma -p luma-menubar

rm -rf "$APP"
mkdir -p "$MACOS"
cp target/release/luma-menubar "$MACOS/luma-menubar"
cp target/release/luma "$MACOS/luma"
chmod 0755 "$MACOS/luma-menubar" "$MACOS/luma"
cp "$ROOT/rust/scripts/menubar-Info.plist" "$CONTENTS/Info.plist"

if [[ ! -x "/usr/bin/codesign" ]]; then
    echo "codesign is required to build the menu-bar app bundle" >&2
    exit 1
fi

# Sign nested executables first, then bind the stable bundle identity to the app itself.
# The default ad-hoc identity is sufficient for local TCC/Login Item testing; set
# CODESIGN_IDENTITY to use a local certificate when one is available.
/usr/bin/codesign --force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_IDENTIFIER.cli" \
    --timestamp=none "$MACOS/luma"
/usr/bin/codesign --force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_IDENTIFIER" \
    --timestamp=none "$MACOS/luma-menubar"
/usr/bin/codesign --force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_IDENTIFIER" \
    --timestamp=none "$APP"
/usr/bin/codesign --verify --deep --strict "$APP"

echo "built and signed $APP (identity: $SIGNING_IDENTITY, identifier: $BUNDLE_IDENTIFIER)"
