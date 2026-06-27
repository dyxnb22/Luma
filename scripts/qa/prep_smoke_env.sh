#!/bin/zsh
# Seed Luma config and browser state for screenshot smoke tests.
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$ROOT/build/Luma.app"
SUPPORT="$HOME/Library/Application Support/Luma"

mkdir -p "$SUPPORT"

# Projects: scan the Luma repo itself (Package.swift + .git).
cat > "$SUPPORT/projects.json" <<EOF
{
  "roots": ["$ROOT"],
  "projects": [],
  "recent": []
}
EOF

# Browser Tabs is default-off; enable it alongside other active modules for QA.
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

defaults write app.luma lumaQASkipRestore -bool true 2>/dev/null || true
defaults write app.luma latencyHUDEnabled -bool true 2>/dev/null || true
defaults write app.luma launcherLastQuery "" 2>/dev/null || true
defaults delete app.luma launcherLastModuleID 2>/dev/null || true

# Open a GitHub tab in Safari so `tab github` has a cached target.
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

# Restart Luma so enabled modules + projects.json are picked up at warmup.
pkill -x Luma 2>/dev/null || true
sleep 0.5
if [[ -d "$APP" ]]; then
  open "$APP"
  sleep 4
else
  echo "warn: $APP not found — run ./scripts/build_app.sh first" >&2
fi

echo "Smoke env ready (projects.json + browser tabs + Safari GitHub tab)"
