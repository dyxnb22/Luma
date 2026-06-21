#!/usr/bin/env bash
set -euo pipefail

PLIST="$HOME/Library/LaunchAgents/app.luma.plist"

launchctl bootout "gui/$(id -u)/app.luma" 2>/dev/null || true
rm -f "$PLIST"
echo "Removed login item"
