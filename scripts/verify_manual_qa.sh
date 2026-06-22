#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Automated QA gate =="
swift build
swift test

echo ""
echo "== Manual QA checklist =="
echo "See docs/MANUAL_QA_CHECKLIST.md for interactive checks."
echo "Sections: Hotkey, Panel, Performance, Permissions, Clipboard, Dashboard (7 cards), Modules (11), Events, Latency HUD, Activity settings."
