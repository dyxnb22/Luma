#!/bin/zsh
# Historical Round 1 replay — same cases as final smoke, without env prep.
exec "$(cd "$(dirname "$0")" && pwd)/run_full_smoke.sh" qa/round-1 --no-prep --sleep-ms 450
