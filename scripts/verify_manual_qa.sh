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
echo "For Auto Workflow final UI acceptance, run:"
echo "  ./scripts/qa/autoworkflow_preflight.sh"
echo "  docs/qa/AUTOWORKFLOW_UI_ACCEPTANCE.md"
echo "  ./scripts/qa/autoworkflow_collect.sh"
echo "Focus on: Hotkey, Panel, Performance, Permissions, Launcher Home/Results, module detail flows, default-off modules, visual polish, Auto Workflow real start/stop/resume, and recorded-review notes."
