#!/usr/bin/env bash
# Measures cold-start time for `swift run Luma` (build + launch until process exits or timeout).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "Building release..."
swift build -c release 2>&1 | tail -3

BIN=".build/release/Luma"
if [[ ! -x "$BIN" ]]; then
  echo "Binary not found at $BIN"
  exit 1
fi

echo "Measuring launch (5s timeout)..."
START=$(python3 -c 'import time; print(time.time())')
timeout 5 "$BIN" >/dev/null 2>&1 || true
END=$(python3 -c 'import time; print(time.time())')
ELAPSED=$(python3 -c "print(f'{$END - $START:.3f}')")
echo "Cold launch window: ${ELAPSED}s (target < 1s after modules ready)"
