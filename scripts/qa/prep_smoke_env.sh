#!/bin/zsh
# Seed Luma config and browser state for screenshot smoke tests.
# Backs up the current app.luma defaults and projects.json once to ~/.qa-backup;
# run restore_smoke_env.sh to undo.
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$ROOT/build/Luma.app"
APP_BINARY="$APP/Contents/MacOS/Luma"
SUPPORT="$HOME/Library/Application Support/Luma"
BACKUP_DIR="$HOME/.qa-backup"
BACKUP_MARKER="$BACKUP_DIR/luma-smoke.exists"
BACKUP_PROJECTS="$BACKUP_DIR/luma-projects.json"
BACKUP_DEFAULTS="$BACKUP_DIR/luma-defaults.plist"

mkdir -p "$SUPPORT"

if [[ ! -f "$BACKUP_MARKER" ]]; then
  mkdir -p "$BACKUP_DIR"
  [[ -f "$SUPPORT/projects.json" ]] && cp "$SUPPORT/projects.json" "$BACKUP_PROJECTS"
  defaults export app.luma "$BACKUP_DEFAULTS" 2>/dev/null || true
  touch "$BACKUP_MARKER"
  echo "Backed up QA state to $BACKUP_DIR"
fi

cat > "$SUPPORT/projects.json" <<EOF
{
  "roots": ["$ROOT"],
  "projects": [],
  "recent": []
}
EOF

defaults write app.luma enabledModules -array \
  "luma.apps" \
  "luma.browser-tabs" \
  "luma.clipboard" \
  "luma.commands" \
  "luma.kill-process" \
  "luma.media" \
  "luma.menu-items" \
  "luma.notes" \
  "luma.projects" \
  "luma.quicklinks" \
  "luma.secrets" \
  "luma.snippets" \
  "luma.todo" \
  "luma.translate" \
  "luma.window-layouts" \
  "luma.wordbook"

defaults write app.luma latencyHUDEnabled -bool true 2>/dev/null || true
defaults write app.luma launcherLastQuery "" 2>/dev/null || true
defaults delete app.luma launcherLastModuleID 2>/dev/null || true

osascript <<'APPLESCRIPT' 2>/dev/null || true
tell application "Safari"
  activate
  if (count of windows) = 0 then
    make new document with properties {URL:"https://github.com"}
  else
    tell front window
      set current tab to (make new tab with properties {URL:"https://github.com"})
    end tell
  end if
end tell
APPLESCRIPT
sleep 2

pkill -x Luma 2>/dev/null || true
sleep 0.5
if [[ -d "$APP" ]]; then
  if [[ -x "$APP_BINARY" ]]; then
    nohup env LUMA_QA=1 "$APP_BINARY" >/dev/null 2>&1 &
  else
    echo "warn: $APP_BINARY not found — falling back to open without QA env" >&2
    open "$APP"
  fi
  sleep 4
else
  echo "warn: $APP not found — run ./scripts/build_app.sh first" >&2
fi

echo "Smoke env ready (projects.json + browser tabs + Safari GitHub tab)"
echo "Restore with: ./scripts/qa/restore_smoke_env.sh"
