#!/usr/bin/env bash
# Phase 22.3 — minimal signed-app keyboard flow smoke.
# Usage: ./scripts/qa/run_keyboard_flows.sh [path/to/Luma.app]
#
# Requires: built signed app, Accessibility permission for Terminal/Cursor,
# jq, osascript. Artifacts: ~/Library/Logs/Luma/keyboard-flows/
# Notes config + app.luma defaults: backed up to ~/.qa-backup/ once per run; restored on EXIT.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
LOG_DIR="${HOME}/Library/Logs/Luma"
ARTIFACT_DIR="$LOG_DIR/keyboard-flows"
STATE_FILE="$LOG_DIR/launcher-state.json"
VIOLATIONS_FILE="$LOG_DIR/launcher-state-violations.json"
SUMMARY_FILE="$ARTIFACT_DIR/summary.json"
IPS_DIR="${HOME}/Library/Logs/DiagnosticReports"
APP="${1:-${LUMA_APP:-$ROOT/build/Luma.app}}"
BIN="$APP/Contents/MacOS/Luma"
BACKUP_DIR="${HOME}/.qa-backup"
KEYBOARD_BACKUP_MARKER="$BACKUP_DIR/luma-keyboard-flows.exists"
KEYBOARD_BACKUP_NOTES="$BACKUP_DIR/luma-keyboard-flows-notes.json"
KEYBOARD_BACKUP_DEFAULTS="$BACKUP_DIR/luma-keyboard-flows-defaults.plist"
QA_NOTES_ROOT="${HOME}/.qa-luma-notes-keyboard"
NOTES_CONFIG="${HOME}/Library/Application Support/Luma/notes.json"
NOTES_RESTORED=0

backup_notes_config() {
  mkdir -p "$BACKUP_DIR" "${HOME}/Library/Application Support/Luma"
  if [[ ! -f "$KEYBOARD_BACKUP_MARKER" ]]; then
    if [[ -f "$NOTES_CONFIG" ]]; then
      cp "$NOTES_CONFIG" "$KEYBOARD_BACKUP_NOTES"
    fi
    defaults export app.luma "$KEYBOARD_BACKUP_DEFAULTS" 2>/dev/null || true
    touch "$KEYBOARD_BACKUP_MARKER"
  fi
}

seed_qa_defaults() {
  defaults write app.luma enabledModules -array \
    "luma.apps" \
    "luma.clipboard" \
    "luma.notes" \
    "luma.quicklinks" \
    "luma.snippets" \
    "luma.todo" \
    "luma.translate"
  defaults write app.luma launcherLastQuery "" 2>/dev/null || true
  defaults delete app.luma launcherLastModuleID 2>/dev/null || true
}

restore_notes_config() {
  if [[ "$NOTES_RESTORED" -eq 1 ]]; then
    return 0
  fi
  NOTES_RESTORED=1
  if [[ ! -f "$KEYBOARD_BACKUP_MARKER" ]]; then
    return 0
  fi
  if [[ -f "$KEYBOARD_BACKUP_NOTES" ]]; then
    cp "$KEYBOARD_BACKUP_NOTES" "$NOTES_CONFIG"
  else
    rm -f "$NOTES_CONFIG"
  fi
if [[ -f "$KEYBOARD_BACKUP_DEFAULTS" ]]; then
    defaults import app.luma "$KEYBOARD_BACKUP_DEFAULTS"
  else
    defaults delete app.luma enabledModules 2>/dev/null || true
    defaults delete app.luma launcherLastQuery 2>/dev/null || true
    defaults delete app.luma launcherLastModuleID 2>/dev/null || true
  fi
  rm -f "$KEYBOARD_BACKUP_MARKER" "$KEYBOARD_BACKUP_NOTES" "$KEYBOARD_BACKUP_DEFAULTS"
  rm -rf "$QA_NOTES_ROOT"
}

seed_notes_root() {
  backup_notes_config
  seed_qa_defaults
  mkdir -p "$QA_NOTES_ROOT/Inbox"
  cat > "$NOTES_CONFIG" <<EOF
{
  "root": "$QA_NOTES_ROOT",
  "expandedFolders": ["$QA_NOTES_ROOT"],
  "recent": [],
  "inboxFolderName": "Inbox",
  "dailyFolderName": "Daily",
  "templatesFolderName": "_templates",
  "reviewsFolderName": "Reviews"
}
EOF
}

mkdir -p "$ARTIFACT_DIR" "$LOG_DIR"

if [[ ! -x "$BIN" ]]; then
  echo "error: missing executable $BIN — run ./scripts/build_app.sh --no-restart" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq required" >&2
  exit 1
fi

trap restore_notes_config EXIT

count_ips() {
  find "$IPS_DIR" -maxdepth 1 -name 'Luma*.ips' 2>/dev/null | wc -l | tr -d ' '
}

wait_state() {
  "$DRIVE" wait-state
}

capture_artifact() {
  local flow_id="$1"
  local step="$2"
  if [[ -f "$STATE_FILE" ]]; then
    cp "$STATE_FILE" "$ARTIFACT_DIR/${flow_id}-${step}.json"
  fi
  "$DRIVE" screenshot "$ARTIFACT_DIR/${flow_id}-${step}.png" 2>/dev/null || true
}

fail_flow() {
  local flow_id="$1"
  local step="$2"
  local message="$3"
  capture_artifact "$flow_id" "${step}-fail"
  echo "FAIL ${flow_id} (${step}): ${message}" >&2
  RESULTS+=("{\"id\":\"${flow_id}\",\"step\":\"${step}\",\"pass\":false,\"message\":$(jq -Rn --arg m "$message" '$m')}")
  FAILURES=$((FAILURES + 1))
}

pass_flow() {
  local flow_id="$1"
  local step="$2"
  capture_artifact "$flow_id" "${step}-pass"
  echo "OK ${flow_id} (${step})"
  RESULTS+=("{\"id\":\"${flow_id}\",\"step\":\"${step}\",\"pass\":true}")
}

assert_jq_wait() {
  local flow_id="$1"
  local step="$2"
  local expr="$3"
  local tries=0
  while [[ $tries -lt 20 ]]; do
    if wait_state && jq -e "$expr" "$STATE_FILE" >/dev/null 2>&1; then
      pass_flow "$flow_id" "$step"
      return 0
    fi
    sleep 0.4
    tries=$((tries + 1))
  done
  fail_flow "$flow_id" "$step" "assertion failed (timeout): $expr"
  jq '.' "$STATE_FILE" 2>/dev/null | head -40 >&2 || true
  return 1
}

assert_jq() {
  local flow_id="$1"
  local step="$2"
  local expr="$3"
  if ! wait_state; then
    fail_flow "$flow_id" "$step" "launcher-state.json missing"
    return 1
  fi
  if jq -e "$expr" "$STATE_FILE" >/dev/null 2>&1; then
    pass_flow "$flow_id" "$step"
    return 0
  fi
  fail_flow "$flow_id" "$step" "assertion failed: $expr"
  jq '.' "$STATE_FILE" 2>/dev/null | head -40 >&2 || true
  return 1
}

start_app() {
  echo "==> starting Luma (LUMA_QA=1)"
  pkill -x Luma 2>/dev/null || true
  sleep 0.6
  rm -f "$STATE_FILE" "$VIOLATIONS_FILE"
  env LUMA_QA=1 LUMA_QA_EXPORT_STATE=1 "$BIN" &
  APP_PID=$!
  sleep 5
  if ! pgrep -x Luma >/dev/null 2>&1; then
    echo "error: Luma process did not start" >&2
    exit 1
  fi
}

luma_windows() {
  osascript -e 'tell application "System Events"
    if not (exists process "Luma") then return 0
    return count of windows of process "Luma"
  end tell' 2>/dev/null || echo 0
}

open_panel() {
  "$DRIVE" open
  local tries=0
  while [[ "$(luma_windows)" -eq 0 ]] && [[ $tries -lt 12 ]]; do
    sleep 0.5
    tries=$((tries + 1))
  done
  if [[ "$(luma_windows)" -eq 0 ]]; then
    echo "warn: launcher panel window not detected (check hotkey / Accessibility)" >&2
  fi
  sleep 0.4
}

stop_app() {
  pkill -x Luma 2>/dev/null || true
  sleep 0.4
}

IPS_BEFORE="$(count_ips)"
FAILURES=0
RESULTS=()
APP_PID=""

seed_notes_root
start_app

# KF-01: open → Esc closes
echo "==> KF-01"
open_panel
assert_jq "KF-01" "open" '.panel.visibilitySessionVisible == true' || true
"$DRIVE" esc
sleep 0.5
assert_jq "KF-01" "esc" '.panel.visibilitySessionVisible == false' || true

start_app

# KF-02: open → n → Return → detail → Esc
echo "==> KF-02"
open_panel
"$DRIVE" bare-open "n"
sleep 2.0
assert_jq_wait "KF-02" "detail-open" \
  '.content.showingDetail == true and .content.currentDetailModuleID == "luma.notes" and .chrome.splitRightPane == "detail"' || true
"$DRIVE" esc
sleep 0.6
assert_jq "KF-02" "detail-esc" \
  '.search.isDetailModeActive == false and .panel.visibilitySessionVisible == true' || true

start_app

# KF-03: clipboard targeted query + results (detail open: ClipboardProductionSmoke; row Return flaky in AX)
echo "==> KF-03"
open_panel
sleep 8
"$DRIVE" type "cb"
sleep 1.2
assert_jq "KF-03" "clipboard-results" \
  '.search.visibleQuery == "cb" and .content.showingResults == true and (.content.selectedItemID | startswith("luma.clipboard:"))' || true
"$DRIVE" return
sleep 1.0
assert_jq "KF-03" "detail-or-results" \
  '.content.showingDetail == true or .content.showingResults == true' || true
"$DRIVE" esc
sleep 0.6
assert_jq "KF-03" "detail-esc" '.search.isDetailModeActive == false' || true

start_app

# KF-04: n detail → Cmd+Space hide → reopen → no placeholder+guide mix
echo "==> KF-04"
open_panel
"$DRIVE" bare-open "n"
sleep 1.5
"$DRIVE" cmd-space
sleep 0.7
"$DRIVE" cmd-space
sleep 0.8
assert_jq "KF-04" "reopen" \
  '(.search.isDetailModeActive == false) or (.chrome.splitRightPane == "detail")' || true

stop_app

# KF-05: menu Show → query → Esc
echo "==> KF-05"
start_app
pkill -x Luma 2>/dev/null || true
sleep 0.6
env LUMA_QA=1 LUMA_QA_EXPORT_STATE=1 "$BIN" &
sleep 3
"$DRIVE" menu-show
sleep 0.8
"$DRIVE" type "safari"
sleep 0.5
"$DRIVE" esc
sleep 0.5
assert_jq "KF-05" "esc-results" \
  '.content.modeKind == "home" or .content.modeKind == "results"' || true
"$DRIVE" esc
sleep 0.5
assert_jq "KF-05" "esc-close" '.panel.visibilitySessionVisible == false' || true

stop_app

IPS_AFTER="$(count_ips)"
IPS_DELTA=$((IPS_AFTER - IPS_BEFORE))

generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
results_json="[]"
if ((${#RESULTS[@]} > 0)); then
  results_json="$(printf '%s\n' "${RESULTS[@]}" | jq -s '.')"
fi
jq -n \
  --arg generatedAt "$generated_at" \
  --argjson failures "$FAILURES" \
  --argjson ipsBefore "$IPS_BEFORE" \
  --argjson ipsAfter "$IPS_AFTER" \
  --argjson ipsDelta "$IPS_DELTA" \
  --argjson results "$results_json" \
  '{generatedAt: $generatedAt, failures: $failures, ipsBefore: $ipsBefore, ipsAfter: $ipsAfter, ipsDelta: $ipsDelta, results: $results}' \
  > "$SUMMARY_FILE"

echo ""
echo "Summary: $SUMMARY_FILE"
echo "Failures: $FAILURES"
echo "IPS delta: $IPS_DELTA ($IPS_BEFORE → $IPS_AFTER)"

if [[ "$FAILURES" -gt 0 ]]; then
  echo "keyboard-flows FAILED" >&2
  exit 1
fi

if [[ "$IPS_DELTA" -gt 0 ]]; then
  echo "warning: new .ips reports detected during keyboard flows" >&2
  exit 1
fi

echo "keyboard-flows PASS"
exit 0
