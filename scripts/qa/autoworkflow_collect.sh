#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="$ROOT/qa/autoworkflow-ui-$STAMP"
STATE_ROOT="${LUMA_AW_STATE_ROOT:-$(defaults read app.luma aw_stateRoot 2>/dev/null || printf "%s/.cc-loop" "$HOME")}"
GUI_PATH="$PATH:/opt/homebrew/bin:/opt/homebrew/anaconda3/bin:/usr/local/bin:/usr/local/anaconda3/bin:$HOME/.local/bin:$HOME/anaconda3/bin:$HOME/bin:/usr/bin:/bin:/usr/sbin:/sbin"

mkdir -p "$OUT_DIR"

write_section() {
  local title="$1"
  local file="$2"
  {
    echo "## $title"
    echo ""
    cat
    echo ""
  } >> "$file"
}

REPORT="$OUT_DIR/report.md"
{
  echo "# Auto Workflow UI Acceptance Evidence"
  echo ""
  echo "- Date: $(date)"
  echo "- Repo: $ROOT"
  echo "- App: $ROOT/build/Luma.app"
  echo "- State root: $STATE_ROOT"
  echo ""
} > "$REPORT"

{
  echo "aw_path=$(defaults read app.luma aw_path 2>/dev/null || true)"
  echo "aw_stateRoot=$(defaults read app.luma aw_stateRoot 2>/dev/null || true)"
  echo "aw_planner=$(defaults read app.luma aw_planner 2>/dev/null || true)"
  echo "aw_reviewer=$(defaults read app.luma aw_reviewer 2>/dev/null || true)"
  echo "aw_implementer=$(defaults read app.luma aw_implementer 2>/dev/null || true)"
  echo "aw_model=$(defaults read app.luma aw_model 2>/dev/null || true)"
  echo ""
  defaults read app.luma enabledModules 2>/dev/null || true
} > "$OUT_DIR/defaults.txt"

ps -axo pid,ppid,stat,command | grep -E 'Luma|cc-loop|autoworkflow' | grep -v grep > "$OUT_DIR/processes.txt" || true

if PATH="$GUI_PATH" command -v cc-loop >/dev/null 2>&1; then
  PATH="$GUI_PATH" cc-loop --help > "$OUT_DIR/cc-loop-help.txt" 2>&1 || true
  PATH="$GUI_PATH" cc-loop list --json > "$OUT_DIR/cc-loop-list.json" 2>"$OUT_DIR/cc-loop-list.stderr" || true
else
  echo "cc-loop not found on Luma GUI PATH" > "$OUT_DIR/cc-loop-help.txt"
  echo "cc-loop not found on Luma GUI PATH" > "$OUT_DIR/cc-loop-list.json"
fi

if [[ -d "$STATE_ROOT" ]]; then
  find "$STATE_ROOT" -maxdepth 3 -type f \( -name 'state.json' -o -name 'runner.pid' -o -name '*.log' \) -print > "$OUT_DIR/state-files.txt" 2>/dev/null || true
else
  echo "State root missing: $STATE_ROOT" > "$OUT_DIR/state-files.txt"
fi

write_section "Defaults" "$REPORT" < "$OUT_DIR/defaults.txt"
write_section "Processes" "$REPORT" < "$OUT_DIR/processes.txt"
write_section "cc-loop list" "$REPORT" < "$OUT_DIR/cc-loop-list.json"
write_section "State files" "$REPORT" < "$OUT_DIR/state-files.txt"

cat <<EOF
Collected Auto Workflow UI QA evidence:
$OUT_DIR

Primary report:
$REPORT
EOF
