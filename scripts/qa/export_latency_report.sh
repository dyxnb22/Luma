#!/usr/bin/env bash
# Validate ~/Library/Logs/Luma/latency-report.json against p95 budgets.
#
# Modes:
#   default          — engineering targets (50 ms hotkey, 30 ms keystroke)
#   LUMA_RELEASE_GATE=1 — RC ceilings (1000 ms hotkey, 60 ms keystroke)
#
# Override: LUMA_LATENCY_HOTKEY_P95_MS, LUMA_LATENCY_KEYSTROKE_P95_MS
set -euo pipefail

REPORT="${HOME}/Library/Logs/Luma/latency-report.json"

if [[ "${LUMA_RELEASE_GATE:-}" == "1" ]]; then
  HOTKEY_BUDGET_MS="${LUMA_LATENCY_HOTKEY_P95_MS:-1000}"
  KEYSTROKE_BUDGET_MS="${LUMA_LATENCY_KEYSTROKE_P95_MS:-60}"
  MODE="release-gate"
else
  HOTKEY_BUDGET_MS="${LUMA_LATENCY_HOTKEY_P95_MS:-50}"
  KEYSTROKE_BUDGET_MS="${LUMA_LATENCY_KEYSTROKE_P95_MS:-30}"
  MODE="engineering"
fi

if [[ ! -f "${REPORT}" ]]; then
  echo "Missing latency report at ${REPORT}." >&2
  echo "Run signed app with LUMA_QA=1, exercise hotkey/keystroke paths, then quit." >&2
  echo "See docs/QA.md § Performance Gate." >&2
  exit 1
fi

echo "Mode: ${MODE} (hotkey ≤ ${HOTKEY_BUDGET_MS} ms, keystroke ≤ ${KEYSTROKE_BUDGET_MS} ms)"

python3 - "$REPORT" "$HOTKEY_BUDGET_MS" "$KEYSTROKE_BUDGET_MS" <<'PY'
import json
import sys

path, hotkey_budget, keystroke_budget = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
with open(path, "r", encoding="utf-8") as handle:
    report = json.load(handle)

hotkey = report.get("hotkeyP95Milliseconds", 0)
keystroke = report.get("keystrokeP95Milliseconds", 0)
combined = report.get("combinedP95Milliseconds", 0)
print(f"hotkey p95={hotkey:.1f}ms (budget {hotkey_budget:.0f}ms)")
print(f"keystroke p95={keystroke:.1f}ms (budget {keystroke_budget:.0f}ms)")
print(f"combined p95={combined:.1f}ms (informational; not gated)")

failed = False
if hotkey > hotkey_budget:
    print("FAIL: hotkey p95 over budget", file=sys.stderr)
    failed = True
if keystroke > keystroke_budget:
    print("FAIL: keystroke p95 over budget", file=sys.stderr)
    failed = True
sys.exit(1 if failed else 0)
PY
