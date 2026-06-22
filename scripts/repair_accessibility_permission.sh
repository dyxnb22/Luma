#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT/build/Luma.app"

if [[ ! -d "$APP_DIR" ]]; then
  echo "Missing $APP_DIR — run ./scripts/build_app.sh first" >&2
  exit 1
fi

pkill -x Luma 2>/dev/null || true
tccutil reset Accessibility app.luma || true
open "$APP_DIR"
open 'x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility'

echo "Reset Accessibility trust for app.luma."
echo "In System Settings, turn Luma on for Accessibility. If it is already on, toggle it off and on."
