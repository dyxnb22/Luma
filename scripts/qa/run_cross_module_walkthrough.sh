#!/usr/bin/env bash
# Cross-module + Translate + Todo follow-up walkthrough (screenshot-driven).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DRIVE="$ROOT/scripts/qa/drive.sh"
SHOTS="$ROOT/qa/round-3/cross-module"
mkdir -p "$SHOTS"

shot() {
  local name="$1"
  screencapture -x -o "$SHOTS/$name.png"
  echo "  screenshot: $name"
}

open_launcher() {
  "$DRIVE" close
  sleep 0.4
  "$DRIVE" open
  sleep 0.6
}

type_query() {
  "$DRIVE" type "$1"
  sleep "${2:-1.2}"
}

echo "== Cross-module walkthrough =="
echo "Output: $SHOTS"

"$ROOT/scripts/qa/prep_smoke_env.sh" | tail -1

# --- Translate detail ---
echo ""
echo "1) Translate: tr hello → detail"
open_launcher
type_query "tr hello" 1.5
shot "01-tr-hello-results"
"$DRIVE" return
sleep 2.0
shot "02-tr-hello-detail"
# Retry translate in detail (Return on focused panel may not apply; user uses Translate button via UI)
"$DRIVE" close
sleep 0.4

# --- Todo detail add/delete ---
echo ""
echo "2) Todo: open detail, add task"
open_launcher
type_query "t" 1.5
shot "03-todo-results"
"$DRIVE" return
sleep 1.5
shot "04-todo-detail-initial"
# Paste unique task title via paste_query
UNIQUE="Luma QA todo $(date +%H%M%S)"
"$ROOT/scripts/qa/paste_query.swift" "$UNIQUE" 2>/dev/null || true
sleep 0.3
osascript -e 'tell application "System Events" to tell process "Luma" to key code 36' 2>/dev/null || true
sleep 1.5
shot "05-todo-after-add"
"$DRIVE" close
sleep 0.4

# --- Clipboard → Snippet ---
echo ""
echo "3) Clipboard → Snippet"
open_launcher
type_query "clip" 1.2
"$DRIVE" return
sleep 1.2
shot "06-clip-detail"
# Open first row action panel / snippet: Tab for secondary on row, or use detail row snippet button via Tab
"$DRIVE" tab
sleep 0.6
shot "07-clip-action-panel"
"$DRIVE" esc
sleep 0.3
# Save as snippet via contextual flow: empty query home suggestion or detail snippet icon
# Use home suggestion after seeding clipboard text
echo "Luma cross-module snippet seed $(date +%s)" | pbcopy
"$DRIVE" close
sleep 0.4
open_launcher
sleep 1.0
shot "08-home-snippet-suggestion"
# Run first suggested row if present (Return)
osascript -e 'tell application "System Events" to tell process "Luma" to key code 36' 2>/dev/null || true
sleep 1.5
shot "09-snippets-after-clipboard-draft"
"$DRIVE" close
sleep 0.4

# --- Translate → Note ---
echo ""
echo "4) Translate → Note"
open_launcher
type_query "tr hello" 1.5
"$DRIVE" return
sleep 2.0
shot "10-translate-detail-for-note"
# Append to note requires successful translation; capture state regardless
"$DRIVE" close

echo ""
echo "Done — screenshots in $SHOTS"
