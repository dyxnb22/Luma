#!/usr/bin/env bash
# Terminable P0 signed-app smoke runner (Phase 15 P2.5).
# Polls ~/Library/Logs/Luma/*-smoke.json (and diagnostics.json for EXPORT).
#
# Usage:
#   ./scripts/run_p0_smokes.sh [path/to/Luma.app]
#   LUMA_APP=/path/to/Luma.app ./scripts/run_p0_smokes.sh
#
# Requires a built signed app (see docs/QA.md § P0 MVP Smoke Gate).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="${HOME}/Library/Logs/Luma"
TIMEOUT_SEC="${LUMA_QA_TIMEOUT_SEC:-90}"
APP="${1:-${LUMA_APP:-}}"

if [[ -z "$APP" ]]; then
  for candidate in \
    "$ROOT/build/Luma.app" \
    "$ROOT/.build/Luma.app" \
    "$ROOT/dist/Luma.app"; do
    if [[ -d "$candidate" ]]; then
      APP="$candidate"
      break
    fi
  done
fi

if [[ -z "$APP" || ! -d "$APP" ]]; then
  echo "error: Luma.app not found. Pass path or set LUMA_APP." >&2
  exit 1
fi

BIN="$APP/Contents/MacOS/Luma"
if [[ ! -x "$BIN" ]]; then
  echo "error: missing executable: $BIN" >&2
  exit 1
fi

mkdir -p "$LOG_DIR"

run_smoke() {
  local env_name="$1"
  local artifact="$2"
  local artifact_path="$LOG_DIR/$artifact"

  echo "==> $env_name → $artifact"
  pkill -x Luma 2>/dev/null || true
  sleep 0.5
  rm -f "$artifact_path"

  env LUMA_QA_AUTO_EXIT=1 "${env_name}=1" "$BIN" &
  local pid=$!
  local elapsed=0
  while (( elapsed < TIMEOUT_SEC )); do
    if [[ -f "$artifact_path" ]]; then
      wait "$pid" 2>/dev/null || true
      echo "    OK $artifact_path"
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      if [[ -f "$artifact_path" ]]; then
        echo "    OK $artifact_path"
        return 0
      fi
      echo "    FAIL $env_name: process exited before $artifact" >&2
      return 1
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  kill "$pid" 2>/dev/null || true
  echo "    FAIL $env_name: timeout after ${TIMEOUT_SEC}s waiting for $artifact" >&2
  return 1
}

failures=0

run_smoke LUMA_QA_APPS apps-smoke.json || failures=$((failures + 1))
run_smoke LUMA_QA_CLIPBOARD clipboard-smoke.json || failures=$((failures + 1))
run_smoke LUMA_QA_NOTES notes-smoke.json || failures=$((failures + 1))
run_smoke LUMA_QA_SETTINGS settings-smoke.json || failures=$((failures + 1))
run_smoke LUMA_QA_EXPORT diagnostics.json || failures=$((failures + 1))

if (( failures > 0 )); then
  echo "P0 smoke runner: $failures failure(s)" >&2
  exit 1
fi

echo "P0 smoke runner: all artifacts present under $LOG_DIR"
exit 0
