#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Automated QA gate =="
swift build
./scripts/test_unit.sh

echo ""
echo "== Manual QA checklist =="
echo "See docs/MANUAL_QA_CHECKLIST.md for interactive checks."
echo "Focus on: Hotkey, Panel, Performance, Permissions, Launcher Home/Results, module detail flows, default-off modules, visual polish, and recorded-review notes."
