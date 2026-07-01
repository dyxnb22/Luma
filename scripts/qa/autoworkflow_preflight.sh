#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP_DIR="$ROOT/build/Luma.app"
APP_BINARY="$APP_DIR/Contents/MacOS/Luma"
QA_SOURCE="${LUMA_AW_QA_SOURCE:-/tmp/luma-aw-qa-source}"
MODULE_ID="luma.autoworkflow"

GUI_PATH="$PATH:/opt/homebrew/bin:/opt/homebrew/anaconda3/bin:/usr/local/bin:/usr/local/anaconda3/bin:$HOME/.local/bin:$HOME/anaconda3/bin:$HOME/bin:/usr/bin:/bin:/usr/sbin:/sbin"

enable_module=0
open_app=0

for arg in "$@"; do
  case "$arg" in
    --enable-module)
      enable_module=1
      ;;
    --open-app)
      open_app=1
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: scripts/qa/autoworkflow_preflight.sh [--enable-module] [--open-app]

Checks the local Auto Workflow QA environment without changing Luma settings by
default. Use --enable-module to append luma.autoworkflow to enabledModules.
USAGE
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

read_default() {
  local key="$1"
  local fallback="$2"
  defaults read app.luma "$key" 2>/dev/null || printf "%s\n" "$fallback"
}

print_check() {
  local label="$1"
  local value="$2"
  printf "%-28s %s\n" "$label" "$value"
}

ensure_qa_source() {
  mkdir -p "$QA_SOURCE"
  if [[ ! -f "$QA_SOURCE/README.md" ]]; then
    cat > "$QA_SOURCE/README.md" <<'EOF'
# Luma Auto Workflow QA Source

Temporary source directory for the manual Auto Workflow UI acceptance pass.
EOF
  fi
  if ! git -C "$QA_SOURCE" rev-parse --is-inside-work-tree &>/dev/null; then
    git -C "$QA_SOURCE" init -b main
    git -C "$QA_SOURCE" add README.md
    git -C "$QA_SOURCE" -c user.name='Luma QA' -c user.email='luma-qa@example.invalid' \
      commit -m 'Initialize Luma Auto Workflow QA source'
  elif ! git -C "$QA_SOURCE" rev-parse --verify main &>/dev/null; then
    git -C "$QA_SOURCE" branch -m main
  fi
}

module_enabled() {
  defaults read app.luma enabledModules 2>/dev/null | grep -q "\"$MODULE_ID\""
}

enable_autoworkflow_module() {
  local raw
  raw="$(defaults read app.luma enabledModules 2>/dev/null | sed -n 's/.*"\(luma\.[^"]*\)".*/\1/p' || true)"
  if [[ -z "$raw" ]]; then
    raw=$'luma.apps\nluma.clipboard\nluma.kill-process\nluma.menu-items\nluma.notes\nluma.projects\nluma.quicklinks\nluma.secrets\nluma.snippets\nluma.todo\nluma.translate\nluma.window-layouts\nluma.wordbook'
  fi
  if ! printf "%s\n" "$raw" | grep -qx "$MODULE_ID"; then
    raw="$(printf "%s\n%s\n" "$raw" "$MODULE_ID" | sed '/^$/d' | sort -u)"
  fi
  # shellcheck disable=SC2207
  local modules=($(printf "%s\n" "$raw"))
  defaults write app.luma enabledModules -array "${modules[@]}"
}

ensure_qa_source

configured_source="$(read_default aw_path "$HOME/autoworkflow")"
state_root="$(read_default aw_stateRoot "$HOME/.cc-loop")"
shell_cc_loop="$(command -v cc-loop 2>/dev/null || true)"
gui_cc_loop="$(PATH="$GUI_PATH" command -v cc-loop 2>/dev/null || true)"

if [[ "$enable_module" -eq 1 ]]; then
  enable_autoworkflow_module
fi

echo "== Auto Workflow UI QA preflight =="
print_check "Repo" "$ROOT"
print_check "Built app" "$APP_DIR"
print_check "App exists" "$( [[ -d "$APP_DIR" ]] && echo yes || echo no )"
print_check "App binary executable" "$( [[ -x "$APP_BINARY" ]] && echo yes || echo no )"
print_check "Configured aw_path" "$configured_source"
print_check "Configured source exists" "$( [[ -d "$configured_source" ]] && echo yes || echo no )"
print_check "QA source" "$QA_SOURCE"
print_check "QA source exists" "$( [[ -d "$QA_SOURCE" ]] && echo yes || echo no )"
print_check "State root" "$state_root"
print_check "State root exists" "$( [[ -d "$state_root" ]] && echo yes || echo no )"
print_check "cc-loop shell PATH" "${shell_cc_loop:-missing}"
print_check "cc-loop Luma GUI PATH" "${gui_cc_loop:-missing}"
print_check "Auto Workflow enabled" "$( module_enabled && echo yes || echo no )"

if [[ -n "$gui_cc_loop" ]]; then
  echo ""
  echo "== cc-loop command surface =="
  PATH="$GUI_PATH" cc-loop --help | sed -n '1,35p'
fi

cat <<EOF

Next manual UI pass:
1. Build with: ./scripts/build_app.sh
2. Open Luma and use Command+Space.
3. In Settings, enable Auto Workflow if the preflight says it is disabled.
4. Use source path: $QA_SOURCE
5. Run the checklist: docs/qa/AUTOWORKFLOW_UI_ACCEPTANCE.md
6. Collect evidence with: ./scripts/qa/autoworkflow_collect.sh
EOF

if [[ "$open_app" -eq 1 ]]; then
  if [[ -d "$APP_DIR" ]]; then
    open "$APP_DIR"
  else
    echo "Cannot open app; build/Luma.app does not exist." >&2
    exit 1
  fi
fi

missing=0
[[ -d "$APP_DIR" ]] || missing=1
[[ -x "$APP_BINARY" ]] || missing=1
[[ -n "$gui_cc_loop" ]] || missing=1
[[ -d "$configured_source" || -d "$QA_SOURCE" ]] || missing=1

exit "$missing"
