#!/bin/zsh
# Ad-hoc single-case smoke helper: run_smoke_case.sh <round_dir> <case_name> <query>
# Full trigger suite: scripts/qa/run_full_smoke.sh (qa/final/run_smoke.sh wraps it)
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
ROUND="$1"
CASE="$2"
QUERY="$3"
SHOTS="$ROUND/screenshots"

"$DRIVE" esc
sleep 0.2
"$DRIVE" hotkey
sleep 0.3
"$DRIVE" screenshot "$SHOTS/${CASE}-01-home.png"
if [[ -n "$QUERY" ]]; then
  "$DRIVE" type "$QUERY"
  sleep 0.35
  "$DRIVE" screenshot "$SHOTS/${CASE}-02-results.png"
fi
