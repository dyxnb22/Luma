#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT/build/Luma.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"
RESTART_AFTER_BUILD=1

for arg in "$@"; do
  case "$arg" in
    --no-restart)
      RESTART_AFTER_BUILD=0
      ;;
    --restart)
      RESTART_AFTER_BUILD=1
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: $0 [--restart|--no-restart]" >&2
      exit 2
      ;;
  esac
done

cd "$ROOT"

if [[ "$RESTART_AFTER_BUILD" -eq 1 ]]; then
  pkill -x Luma 2>/dev/null || true
fi

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
      <key>NSRemindersFullAccessUsageDescription</key>
      <string>Luma needs Reminders access to create reminders and show your Todo list.</string>
      <key>NSRemindersUsageDescription</key>
      <string>Luma needs Reminders access to create reminders and show your Todo list.</string>
      <key>NSCalendarsFullAccessUsageDescription</key>
      <string>Luma needs Calendar access to create events and show today's schedule.</string>
      <key>NSCalendarsUsageDescription</key>
      <string>Luma needs Calendar access to create events and show today's schedule.</string>
</dict>
</plist>
EOF

SIGN_IDENTITY="${LUMA_CODESIGN_IDENTITY:-}"
if [[ -z "$SIGN_IDENTITY" ]]; then
  SIGN_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null | awk -F '\"' '/Apple Development|Developer ID Application/ { print $2; exit }')"
fi
if [[ -z "$SIGN_IDENTITY" ]]; then
  SIGN_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null | awk -F '\"' '/Luma Local Development/ { print $2; exit }')"
fi
if [[ -z "$SIGN_IDENTITY" ]]; then
  SIGN_IDENTITY="-"
  echo "No stable code-signing identity found; using ad-hoc signing. Run ./scripts/install_local_codesign_cert.sh to stabilize Accessibility across rebuilds."
else
  echo "Signing with: $SIGN_IDENTITY"
fi

codesign --force --sign "$SIGN_IDENTITY" --deep --options runtime "$APP_DIR"
codesign --verify --verbose=2 "$APP_DIR"

echo "Built and signed: $APP_DIR"

if [[ "$RESTART_AFTER_BUILD" -eq 1 ]]; then
  open "$APP_DIR"
  echo "Restarted Luma"
fi
