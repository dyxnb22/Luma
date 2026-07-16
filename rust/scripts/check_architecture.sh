#!/usr/bin/env bash
# Architecture dependency allowlist for Luma crates.
# Guards module/TUI boundaries; documents allowed edges (TUI → application projections/ports,
# composition → storage/platform adapters).
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
# application → storage is allowed (settings adapters); engine must not open stores directly.
if rg -n 'ClipboardStore::luma_next_default|NotesIndexStore::luma_next_default|QuicklinksStore::luma_next_default|SnippetsStore::luma_next_default' \
  crates/luma-application/src/engine.rs crates/luma-application/src/engine/*.rs 2>/dev/null | head -20 | grep .; then
  echo "FAIL: engine must not open stores directly (use injected repositories in compose.rs)"
  fail=1
fi

if rg -n 'ConfigStore::luma_next_default|MacPasteboard::|MacOpenPath|MacKeychain::' \
  crates/luma-modules/src 2>/dev/null | head -20 | grep .; then
  echo "FAIL: production module sources reference Mac*/ConfigStore constructors"
  fail=1
fi

# Modules use application ports for filesystem, network, shell, and raw platform work. Adapter
# integration tests may use host fixtures, so inspect only the production prefix of each module.
while IFS= read -r module_source; do
  module_production="$(sed '/^#\[cfg(test)\]$/,$d' "$module_source")"
  if printf '%s\n' "$module_production" \
    | rg -n 'use std::(fs|net|process)|std::fs::|tokio::fs::|std::net::|tokio::net::|std::process::Command|tokio::process::Command|Command::new\(|\b(reqwest|ureq)::|\.canonicalize\(|\.exists\(|\.is_dir\(|\.is_file\(|std::env::current_dir\(' \
    | head -20 \
    | grep .; then
    echo "FAIL: module production code must use application ports for host I/O: $module_source"
    fail=1
  fi
done < <(find crates/luma-modules/src -type f -name '*.rs' -print | sort)

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
echo "architecture allowlist OK"
