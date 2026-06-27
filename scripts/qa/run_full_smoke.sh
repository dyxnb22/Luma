#!/bin/zsh
# Full trigger smoke — screenshot-driven QA via drive.sh
# Usage: run_full_smoke.sh [round_dir] [--no-prep] [--sleep-ms N]
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
ROUND="$ROOT/qa/final"
RUN_PREP=1
RESULT_SLEEP="2.0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-prep) RUN_PREP=0; shift ;;
    --sleep-ms)
      RESULT_SLEEP="$(awk "BEGIN { printf \"%.3f\", $2 / 1000 }")"
      shift 2
      ;;
    *)
      if [[ -d "$1" ]]; then
        ROUND="$(cd "$1" && pwd)"
      elif [[ -d "$ROOT/$1" ]]; then
        ROUND="$(cd "$ROOT/$1" && pwd)"
      else
        echo "Unknown argument: $1" >&2
        exit 1
      fi
      shift
      ;;
  esac
done

SHOTS="$ROUND/screenshots"
mkdir -p "$SHOTS"

prep_menu_target() {
  osascript -e 'tell application "Cursor" to activate' 2>/dev/null || true
  sleep 1.5
}

prep_preview() {
  if ! pgrep -x Preview >/dev/null 2>&1; then
    open -a Preview
    sleep 1.2
  fi
}

smoke() {
  local case="$1"
  local query="$2"
  local prep="${3:-}"

  echo "=== $case: '$query' ==="
  [[ -n "$prep" ]] && $prep
  "$DRIVE" close
  sleep 0.5
  "$DRIVE" open
  sleep 0.5
  "$DRIVE" screenshot "$SHOTS/${case}-01-home.png"
  if [[ -n "$query" ]]; then
    "$DRIVE" type "$query"
    sleep "$RESULT_SLEEP"
    "$DRIVE" screenshot "$SHOTS/${case}-02-results.png"
  fi
}

if [[ "$RUN_PREP" -eq 1 ]]; then
  "$ROOT/scripts/qa/prep_smoke_env.sh"
fi

smoke "home" ""
smoke "apps-safari" "Safari"
smoke "clip-empty" "clip"
smoke "clip-luma" "clip luma"
smoke "cmd-settings" "cmd settings"
smoke "note-empty" "note"
smoke "t-empty" "t"
smoke "t-buy" "t buy milk"
smoke "tr-hello" "tr hello world"
smoke "word-empty" "word"
smoke "s-empty" "s"
smoke "sec-empty" "sec"
smoke "m-empty" "m"
smoke "layout-left" "layout left"
smoke "proj-empty" "proj"
smoke "proj-luma" "proj luma"
smoke "gh-swift" "gh swift package"
smoke "mb-empty" "mb"
smoke "mb-fold" "mb zoom" prep_menu_target
smoke "kill-empty" "kill"
smoke "kill-preview" "kill preview" prep_preview
smoke "tab-empty" "tab"
smoke "tab-github" "tab github"
smoke "ql-detail" "quicklinks"

echo "Done - screenshots in $SHOTS"
