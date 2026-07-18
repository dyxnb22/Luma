#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUST_ROOT="$ROOT/rust"
APP="${1:-$RUST_ROOT/target/Luma Menu Bar.app}"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"

cd "$RUST_ROOT"
cargo build --release -p luma -p luma-menubar

rm -rf "$APP"
mkdir -p "$MACOS"
cp target/release/luma-menubar "$MACOS/luma-menubar"
cp target/release/luma "$MACOS/luma"
chmod 0755 "$MACOS/luma-menubar" "$MACOS/luma"
cp "$ROOT/rust/scripts/menubar-Info.plist" "$CONTENTS/Info.plist"

echo "built $APP"
