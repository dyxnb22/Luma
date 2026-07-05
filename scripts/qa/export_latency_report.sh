#!/usr/bin/env bash
set -euo pipefail

REPORT="${HOME}/Library/Logs/Luma/latency-report.json"
HOTKEY_BUDGET_MS="${LUMA_LATENCY_HOTKEY_P95_MS:-50}"
KEYSTROKE_BUDGET_MS="${LUMA_LATENCY_KEYSTROKE_P95_MS:-30}"

if [[ ! -f "${REPORT}" ]]; then
  echo "Missing latency report at ${REPORT}. Run the app with LUMA_QA=1 and exercise hotkey/keystroke paths first." >&2
  exit 1
fi

python3 - "$REPORT" "$HOTKEY_BUDGET_MS" "$KEYSTROKE_BUDGET_MS" <<'PY'
import json
import sys

path, hotkey_budget, keystroke_budget = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
with open(path, "r", encoding="utf-8") as handle:
    report = json.load(handle)

hotkey = report.get("hotkeyP95Milliseconds", 0)
keystroke = report.get("keystrokeP95Milliseconds", 0)
print(f"hotkey p95={hotkey:.1f}ms (budget {hotkey_budget:.0f}ms)")
print(f"keystroke p95={keystroke:.1f}ms (budget {keystroke_budget:.0f}ms)")

failed = False
if hotkey > hotkey_budget:
    print("FAIL: hotkey p95 over budget", file=sys.stderr)
    failed = True
if keystroke > keystroke_budget:
    print("FAIL: keystroke p95 over budget", file=sys.stderr)
    failed = True
sys.exit(1 if failed else 0)
PY
