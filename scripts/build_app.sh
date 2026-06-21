#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT/build/Luma.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"

cd "$ROOT"
swift build -c release

mkdir -p "$MACOS_DIR"
cp "$ROOT/.build/release/Luma" "$MACOS_DIR/Luma"

cat > "$APP_DIR/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleIdentifier</key>
	<string>app.luma</string>
	<key>CFBundleName</key>
	<string>Luma</string>
	<key>CFBundleExecutable</key>
	<string>Luma</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>LSUIElement</key>
	<true/>
	<key>LSMinimumSystemVersion</key>
	<string>14.0</string>
</dict>
</plist>
EOF

echo "Built $APP_DIR"
