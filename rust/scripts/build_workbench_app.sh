#!/usr/bin/env bash
# Local personal packaging for the native workbench host (ADR-0007).
# Not distribution: no notarization, no DMG, no updater, no release automation.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUST_ROOT="$ROOT/rust"
PACKAGE="$RUST_ROOT/native/luma-workbench"
APP="${1:-$RUST_ROOT/target/Luma.app}"
APP_BASENAME="$(basename "$APP")"
APP_PARENT="$(dirname "$APP")"
if [[ "$APP_BASENAME" != *.app || "$APP_BASENAME" == ".app" ]]; then
    echo "workbench output must be a specifically named .app bundle: $APP" >&2
    exit 1
fi
mkdir -p "$APP_PARENT"
APP_PARENT="$(cd "$APP_PARENT" && pwd -P)"
APP="$APP_PARENT/$APP_BASENAME"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
BUNDLE_IDENTIFIER="com.luma.next.workbench"
SIGNING_IDENTITY="-"
if [[ -n "${CODESIGN_IDENTITY+x}" && -n "$CODESIGN_IDENTITY" ]]; then
    SIGNING_IDENTITY="$CODESIGN_IDENTITY"
elif /usr/bin/security find-identity -v -p codesigning 2>/dev/null \
    | /usr/bin/grep -Fq '"Luma Local Development"'; then
    # This local certificate gives the bundle a stable designated requirement, so a normal
    # rebuild does not discard macOS privacy grants such as Screen Recording.
    SIGNING_IDENTITY="Luma Local Development"
fi

cd "$RUST_ROOT"
cargo build --release -p luma
swift build --package-path "$PACKAGE" -c release
HOST_BIN="$(swift build --package-path "$PACKAGE" -c release --show-bin-path)/LumaWorkbench"

if [[ ! -x "$HOST_BIN" ]]; then
    echo "swift build did not produce $HOST_BIN" >&2
    exit 1
fi

rm -rf "$APP"
mkdir -p "$MACOS"
# Contents/MacOS/LumaWorkbench is the AppKit host; Contents/MacOS/luma is the Rust workbench it
# runs. The host cannot be named "Luma": APFS is case-insensitive by default, so that name and
# "luma" would silently collapse into a single file.
cp "$HOST_BIN" "$MACOS/LumaWorkbench"
cp target/release/luma "$MACOS/luma"
chmod 0755 "$MACOS/LumaWorkbench" "$MACOS/luma"
cp "$RUST_ROOT/scripts/workbench-Info.plist" "$CONTENTS/Info.plist"
/usr/bin/plutil -lint "$CONTENTS/Info.plist" >/dev/null

plist_executable="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$CONTENTS/Info.plist")"
plist_identifier="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$CONTENTS/Info.plist")"
if [[ "$plist_executable" != "LumaWorkbench" || "$plist_identifier" != "$BUNDLE_IDENTIFIER" ]]; then
    echo "workbench Info.plist identity does not match the packaged host" >&2
    exit 1
fi

host_inode="$(stat -f %i "$MACOS/LumaWorkbench")"
cli_inode="$(stat -f %i "$MACOS/luma")"
if [[ "$host_inode" == "$cli_inode" ]]; then
    echo "host and CLI collapsed into one file — check for a case-insensitive name clash" >&2
    exit 1
fi

if [[ ! -x "/usr/bin/codesign" ]]; then
    echo "codesign is required to build the workbench app bundle" >&2
    exit 1
fi

# Sign nested executables first, then bind the bundle identifier to the app itself. The Rust TUI
# calls CoreGraphics itself, so it must share the host's identifier for Screen Recording TCC to
# apply to the PTY child as well as the AppKit process.
# The default ad-hoc identity is enough to make a local bundle runnable, but rebuilding it can
# invalidate TCC grants. Set CODESIGN_IDENTITY to a stable local certificate when continuity
# matters, and re-check module-local permissions after an ad-hoc rebuild.
/usr/bin/codesign --force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_IDENTIFIER" \
    --timestamp=none "$MACOS/luma"
/usr/bin/codesign --force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_IDENTIFIER" \
    --timestamp=none "$MACOS/LumaWorkbench"
/usr/bin/codesign --force --sign "$SIGNING_IDENTITY" --identifier "$BUNDLE_IDENTIFIER" \
    --timestamp=none "$APP"
/usr/bin/codesign --verify --deep --strict "$APP"

echo "built and signed $APP (identity: $SIGNING_IDENTITY, identifier: $BUNDLE_IDENTIFIER)"
