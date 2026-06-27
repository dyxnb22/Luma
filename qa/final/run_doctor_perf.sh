#!/bin/zsh
# Measure keystroke→first-paint via latency HUD overlay
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
SHOTS="$(cd "$(dirname "$0")" && pwd)/screenshots"
mkdir -p "$SHOTS"

"$DRIVE" close
sleep 0.5
"$DRIVE" open
sleep 0.5

for q in "s" "gh swift" "Safari" "clip luma" "tab github" "kill preview"; do
  "$DRIVE" close
  sleep 0.3
  "$DRIVE" open
  sleep 0.4
  "$DRIVE" type "$q"
  sleep 0.8
done

"$DRIVE" screenshot "$SHOTS/doctor-latency-hud.png"
echo "Latency HUD screenshot: $SHOTS/doctor-latency-hud.png"
