#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DO_BUILD=1
DO_SMOKE=1

for arg in "$@"; do
  case "$arg" in
    --no-build)
      DO_BUILD=0
      ;;
    --no-smoke)
      DO_SMOKE=0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: $0 [--no-build] [--no-smoke]" >&2
      exit 2
      ;;
  esac
done

cd "$ROOT"

echo "== Luma recorded review =="

if [[ "$DO_BUILD" -eq 1 ]]; then
  echo ""
  echo "1) Building app"
  ./scripts/build_app.sh
fi

echo ""
echo "2) Preparing QA environment"
./scripts/qa/prep_smoke_env.sh

if [[ "$DO_SMOKE" -eq 1 ]]; then
  echo ""
  echo "3) Running scripted smoke"
  ./scripts/qa/run_full_smoke.sh
fi

echo ""
echo "4) Ready for recorded walkthrough"
echo "Guide:    $ROOT/docs/QA.md"
echo "Shots:    $ROOT/qa/final/screenshots"
echo ""
echo "Suggested flow:"
echo "- Start screen recording"
echo "- Walk home, search, detail, permissions, and cross-module flows"
echo "- Log findings with the format in docs/QA.md"
