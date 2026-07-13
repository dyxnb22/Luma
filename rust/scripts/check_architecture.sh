#!/usr/bin/env bash
# Architecture dependency allowlist for Luma crates.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail=0

check_absent() {
  local pkg="$1"
  shift
  local deps
  deps="$(cargo metadata --format-version 1 --no-deps \
    | python3 -c "
import json,sys
d=json.load(sys.stdin)
p=next(x for x in d['packages'] if x['name']=='$pkg')
print(' '.join(dep['name'] for dep in p['dependencies'] if dep.get('kind') is None))
")"
  for banned in "$@"; do
    if echo " $deps " | grep -q " $banned "; then
      echo "FAIL: $pkg must not depend on $banned (normal deps: $deps)"
      fail=1
    fi
  done
}

check_absent luma-tui luma-platform-macos luma-storage luma-modules
check_absent luma-modules luma-platform-macos luma-storage
check_absent luma-domain luma-platform-macos luma-storage luma-modules luma-tui

if rg -n 'ConfigStore::luma_next_default|MacPasteboard::|MacOpenPath|MacKeychain::' \
  crates/luma-modules/src 2>/dev/null | head -20 | grep .; then
  echo "FAIL: production module sources reference Mac*/ConfigStore constructors"
  fail=1
fi

if rg -n 'Command::new\(' crates/luma-modules/src 2>/dev/null | head -20 | grep .; then
  echo "FAIL: modules must not spawn processes via Command::new (use injected ports)"
  fail=1
fi

if rg -n 'Command::new\("open"\)' crates/luma-modules/src 2>/dev/null | head -20 | grep .; then
  echo "FAIL: modules must use OpenPathPort, not Command::new(\"open\")"
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
echo "architecture allowlist OK"
