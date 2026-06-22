#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT/build/Luma.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"

cd "$ROOT"
swift build -c release

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"
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
      <key>CFBundleDisplayName</key>
      <string>Luma</string>
      <key>CFBundleExecutable</key>
      <string>Luma</string>
      <key>CFBundleShortVersionString</key>
      <string>0.1.0</string>
      <key>CFBundleVersion</key>
      <string>1</string>
      <key>CFBundlePackageType</key>
      <string>APPL</string>
      <key>LSUIElement</key>
      <true/>
      <key>LSMinimumSystemVersion</key>
      <string>14.0</string>
      <key>NSHumanReadableCopyright</key>
      <string>© Luma local user</string>
</dict>
</plist>
EOF

# Ad-hoc sign so the app keeps a stable identity on this machine.
codesign --force --sign - --deep --options runtime "$APP_DIR"
codesign --verify --verbose=2 "$APP_DIR"

echo "Built and ad-hoc signed: $APP_DIR"
