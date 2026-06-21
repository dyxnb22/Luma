#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXECUTABLE="$ROOT/build/Luma.app/Contents/MacOS/Luma"
PLIST="$HOME/Library/LaunchAgents/app.luma.plist"

if [[ ! -x "$EXECUTABLE" ]]; then
  echo "Missing $EXECUTABLE — run ./scripts/build_app.sh first" >&2
  exit 1
fi

mkdir -p "$HOME/Library/LaunchAgents"

cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>app.luma</string>
	<key>ProgramArguments</key>
	<array>
		<string>$EXECUTABLE</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<false/>
</dict>
</plist>
EOF

launchctl bootout "gui/$(id -u)/app.luma" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$PLIST"
echo "Installed login item at $PLIST"
