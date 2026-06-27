#!/bin/zsh
# run_round1_smoke.sh - Run all trigger smoke tests for Round 1
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
ROUND="$ROOT/qa/round-1"
SHOTS="$ROUND/screenshots"
mkdir -p "$SHOTS"

smoke() {
  local case="$1"
  local query="$2"
  echo "=== $case: '$query' ==="
  "$DRIVE" close
  sleep 0.2
  "$DRIVE" open
  sleep 0.35
  "$DRIVE" screenshot "$SHOTS/${case}-01-home.png"
  if [[ -n "$query" ]]; then
    "$DRIVE" type "$query"
    sleep 0.45
    "$DRIVE" screenshot "$SHOTS/${case}-02-results.png"
  fi
}

# A) Cold start home
smoke "home" ""

# B) Historical modules + new modules
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

# New modules
smoke "gh-swift" "gh swift package"
smoke "mb-empty" "mb"
smoke "mb-fold" "mb fold"
smoke "kill-empty" "kill"
smoke "kill-preview" "kill preview"
smoke "tab-empty" "tab"
smoke "tab-github" "tab github"

# Quicklinks detail
smoke "ql-detail" "quicklinks"

echo "Done - screenshots in $SHOTS"
