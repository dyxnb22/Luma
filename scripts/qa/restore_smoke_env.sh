#!/bin/zsh
# Restore app.luma defaults and projects.json captured by prep_smoke_env.sh.
set -e
SUPPORT="$HOME/Library/Application Support/Luma"
BACKUP_DIR="$HOME/.qa-backup"
BACKUP_MARKER="$BACKUP_DIR/luma-smoke.exists"
BACKUP_PROJECTS="$BACKUP_DIR/luma-projects.json"
BACKUP_DEFAULTS="$BACKUP_DIR/luma-defaults.plist"
BACKUP_NOTES="$BACKUP_DIR/luma-notes.json"
NOTES_CONFIG="$SUPPORT/notes.json"
QA_NOTES_ROOT="$HOME/.qa-luma-notes"

if [[ ! -f "$BACKUP_MARKER" ]]; then
  echo "No QA backup found at $BACKUP_DIR" >&2
  exit 0
fi

if [[ -f "$BACKUP_PROJECTS" ]]; then
  cp "$BACKUP_PROJECTS" "$SUPPORT/projects.json"
else
  rm -f "$SUPPORT/projects.json"
fi

if [[ -f "$BACKUP_NOTES" ]]; then
  cp "$BACKUP_NOTES" "$NOTES_CONFIG"
else
  rm -f "$NOTES_CONFIG"
fi

if [[ -f "$BACKUP_DEFAULTS" ]]; then
  defaults import app.luma "$BACKUP_DEFAULTS"
else
  defaults delete app.luma enabledModules 2>/dev/null || true
  defaults delete app.luma latencyHUDEnabled 2>/dev/null || true
  defaults delete app.luma launcherLastQuery 2>/dev/null || true
  defaults delete app.luma launcherLastModuleID 2>/dev/null || true
  defaults delete app.luma lumaQASkipRestore 2>/dev/null || true
fi

rm -f "$BACKUP_MARKER" "$BACKUP_PROJECTS" "$BACKUP_DEFAULTS" "$BACKUP_NOTES"
rmdir "$BACKUP_DIR" 2>/dev/null || true
rm -rf "$QA_NOTES_ROOT"
pkill -x Luma 2>/dev/null || true
sleep 0.5
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if [[ -d "$ROOT/build/Luma.app" ]]; then
  open "$ROOT/build/Luma.app"
fi

echo "Restored QA backup and restarted Luma"
