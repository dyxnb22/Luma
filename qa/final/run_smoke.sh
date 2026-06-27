#!/bin/zsh
exec "$(cd "$(dirname "$0")/../.." && pwd)/scripts/qa/run_full_smoke.sh" "$(cd "$(dirname "$0")" && pwd)"
