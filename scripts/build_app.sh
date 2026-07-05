#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/release/common.sh
source "$ROOT/scripts/release/common.sh"

APP_DIR="$ROOT/build/Luma.app"
RESTART_AFTER_BUILD=1

for arg in "$@"; do
  case "$arg" in
    --no-restart)
      RESTART_AFTER_BUILD=0
      ;;
    --restart)
      RESTART_AFTER_BUILD=1
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: $0 [--restart|--no-restart]" >&2
      exit 2
      ;;
  esac
done

cd "$ROOT"

if [[ "$RESTART_AFTER_BUILD" -eq 1 ]]; then
  pkill -x Luma 2>/dev/null || true
fi

luma_swift_build_release "$ROOT"
luma_assemble_app "$ROOT" "$APP_DIR"

echo "Built and signed: $APP_DIR"

if [[ "$RESTART_AFTER_BUILD" -eq 1 ]]; then
  open "$APP_DIR"
  echo "Restarted Luma"
fi
