#!/usr/bin/env bash
# Recoverable, isolated CLI soak. It deliberately never calls GUI/AX automation.
set -euo pipefail

ROOT="${LUMA_SOAK_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/luma-soak.XXXXXX")}"
STATE="$ROOT/state.tsv"
SUPPORT="$ROOT/support"
LOGS="$ROOT/logs"
ITERATIONS="${1:-1}"
BIN="${LUMA_SOAK_BIN:-cargo run -q -p luma --}"
mkdir -p "$SUPPORT" "$LOGS"
touch "$STATE"

run_case() {
  local name="$1"; shift
  local started ended code
  started="$(date +%s)"
  set +e
  LUMA_NEXT_SUPPORT_DIR="$SUPPORT" LUMA_NEXT_LOGS_DIR="$LOGS" $BIN "$@" >/dev/null 2>"$ROOT/$name.stderr"
  code=$?
  set -e
  ended="$(date +%s)"
  if rg -q "panicked at|thread.*panic" "$ROOT/$name.stderr"; then code=101; fi
  printf '%s\t%s\t%s\t%s\n' "$(date -u +%FT%TZ)" "$name" "$code" "$((ended-started))" >>"$STATE"
}

for ((i=1; i<=ITERATIONS; i++)); do
  run_case doctor doctor --json
  run_case query query app --json
  run_case migrate migrate clipboard-fixture --dry-run --json
done
printf 'state=%s iterations=%s\n' "$STATE" "$ITERATIONS"
