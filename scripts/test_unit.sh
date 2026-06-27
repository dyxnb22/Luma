#!/usr/bin/env bash
# Deterministic unit/regression gate — skips machine-dependent integration tests.
# Run integration suite: LUMA_INTEGRATION_TESTS=1 swift test --filter tag:integration
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
swift test
