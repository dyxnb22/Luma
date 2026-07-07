#!/usr/bin/env bash
# Release Candidate Gate — automates REFACTOR_PLAN P3.4 / docs/QA.md steps 2–8 + latency check.
# Step 1 (git status) and step 9 (manual supplement) are operator responsibility.
#
# Usage: ./scripts/run_release_gate.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
LOG_DIR="${HOME}/Library/Logs/Luma"
IPS_DIR="${HOME}/Library/Logs/DiagnosticReports"

step() { echo ""; echo "==> $*"; }

fail() {
  echo "RELEASE GATE FAIL: $*" >&2
  exit 1
}

count_ips() {
  local n
  n="$(find "${IPS_DIR}" -maxdepth 1 -name 'Luma*.ips' 2>/dev/null | wc -l | tr -d ' ')"
  echo "${n:-0}"
}

step "1/9 (informational) git status"
git status --short || true
git log -1 --oneline || true

step "2/9 swift build"
swift build || fail "swift build"

step "3/9 swift test"
swift test || fail "swift test"

step "4/9 scanners"
bash scripts/scan_handle_memory_only.sh || fail "scan_handle_memory_only"
bash scripts/scan_appkit_executor_risk.sh || fail "scan_appkit_executor_risk"

step "5/9 build signed app"
./scripts/build_app.sh --no-restart || fail "build_app"

IPS_BEFORE="$(count_ips)"
echo "    .ips before: ${IPS_BEFORE}"

step "6/9 run_p0_smokes"
./scripts/run_p0_smokes.sh || fail "run_p0_smokes"

step "7/9 verify smoke artifacts"
for f in apps-smoke.json clipboard-smoke.json notes-smoke.json settings-smoke.json diagnostics.json; do
  if [[ ! -f "${LOG_DIR}/${f}" ]]; then
    fail "missing ${LOG_DIR}/${f}"
  fi
  echo "    OK ${LOG_DIR}/${f}"
done

step "8/9 verify no new .ips"
IPS_AFTER="$(count_ips)"
echo "    .ips after: ${IPS_AFTER}"
if [[ "${IPS_AFTER}" != "${IPS_BEFORE}" ]]; then
  fail ".ips count changed (${IPS_BEFORE} -> ${IPS_AFTER})"
fi

step "9/9 latency-report (release-gate budgets)"
echo "    Note: existing latency-report.json check is mandatory; fresh LUMA_QA=1 session recommended before RC tag (docs/QA.md § Performance Gate)."
LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh || fail "export_latency_report (missing or over-budget latency-report.json)"

if pgrep -x Luma >/dev/null 2>&1; then
  echo "    WARN: Luma process still running (pgrep -x Luma); smokes should have exited"
fi

echo ""
echo "RELEASE GATE PASS (automated steps 2–8 + mandatory latency check)."
echo "Next: complete manual supplement (docs/QA.md § Release Candidate Gate step 9)."
echo "Recommended before RC tag: fresh LUMA_QA=1 latency session (see § Performance Gate)."
exit 0
