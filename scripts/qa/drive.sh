#!/bin/zsh
# drive.sh <action> [args...]
# action in { hotkey, type, key, screenshot, esc, return, tab, cmd-k, clear, focus, open, close }
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

clear_persisted_query() {
  defaults write app.luma launcherLastQuery "" 2>/dev/null || true
  defaults delete app.luma launcherLastModuleID 2>/dev/null || true
}

escape_applescript() {
  print -r -- "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

luma_window_count() {
  osascript -e 'tell application "System Events"
    if not (exists process "Luma") then return 0
    return count of windows of process "Luma"
  end tell' 2>/dev/null || echo 0
}

luma_close() {
  local tries=0
  while [[ "$(luma_window_count)" -gt 0 ]] && [[ $tries -lt 4 ]]; do
    osascript -e 'tell application "System Events" to tell process "Luma" to key code 53' 2>/dev/null || true
    sleep 0.35
    tries=$((tries + 1))
  done
  clear_persisted_query
  sleep 0.15
}

luma_open() {
  clear_persisted_query
  if [[ "$(luma_window_count)" -eq 0 ]]; then
    osascript -e 'tell application "System Events" to key code 49 using {command down}'
    sleep 0.55
  fi
}

set_query() {
  "$SCRIPT_DIR/paste_query.swift" "$@"
}

case "$1" in
  hotkey)
    if [[ "$(luma_window_count)" -gt 0 ]]; then
      luma_close
    else
      luma_open
    fi ;;
  open)
    luma_open
    sleep 0.65
    set_query "" ;;
  close|esc)
    luma_close ;;
  type)
    shift
    set_query "$*"
    sleep 0.55 ;;
  key)
    osascript -e "tell application \"System Events\" to tell process \"Luma\" to key code $2"
    sleep 0.08 ;;
  return)
    osascript -e 'tell application "System Events" to tell process "Luma" to key code 36'
    sleep 0.2 ;;
  tab)
    osascript -e 'tell application "System Events" to tell process "Luma" to key code 48'
    sleep 0.1 ;;
  cmd-k)
    osascript -e 'tell application "System Events" to tell process "Luma" to keystroke "k" using {command down}'
    sleep 0.1 ;;
  clear)
    set_query "" ;;
  screenshot)
    out="$2"
    mkdir -p "$(dirname "$out")"
    screencapture -x -o "$out" ;;
  *) echo "unknown: $1" >&2 ; exit 2 ;;
esac
