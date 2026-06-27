#!/bin/zsh
# Full trigger smoke — screenshot-driven QA via drive.sh
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
SHOTS="$(cd "$(dirname "$0")" && pwd)/screenshots"
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
    sleep 2.0
    "$DRIVE" screenshot "$SHOTS/${case}-02-results.png"
  fi
}

"$ROOT/scripts/qa/prep_smoke_env.sh"

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
