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
check_absent luma objc2 objc2-app-kit objc2-service-management

# ADR-0006 is superseded by ADR-0007. The old Rust/AppKit companion must not quietly return as a
# second native entry point.
if [[ -e "bins/luma-menubar" || -e "scripts/build_menubar_app.sh" || -e "scripts/menubar-Info.plist" ]]; then
  echo "FAIL: the superseded menu-bar companion has been restored without a new ADR"
  fail=1
fi

if rg -n 'objc2(-app-kit|-service-management)?|objc2_foundation' \
  bins/luma/Cargo.toml crates/*/Cargo.toml 2>/dev/null | head -20 | grep .; then
  echo "FAIL: Rust crates must remain free of AppKit/ServiceManagement dependencies"
  fail=1
fi

# ADR-0007: the Swift workbench host has no Rust dependency edges, so its boundary is guarded by
# source inspection. It hosts a PTY; it is not a second UI, a module surface, or a status item.
WORKBENCH_SOURCES="native/luma-workbench/Sources"
if [[ -d "$WORKBENCH_SOURCES" ]]; then
  if rg -n 'import SwiftUI|NSStatusItem|LumaNext|luma-protocol' "$WORKBENCH_SOURCES" \
    2>/dev/null | head -20 | grep .; then
    echo "FAIL: the workbench host must not use SwiftUI, add a status item, or touch LumaNext"
    fail=1
  fi
  if rg -n 'CG(Request|Preflight)ScreenCaptureAccess' "$WORKBENCH_SOURCES" \
    2>/dev/null | head -20 | grep .; then
    echo "FAIL: module permissions must stay in their platform adapter, not the PTY host"
    fail=1
  fi
  # The host owns exactly one child process invocation: the bundled `luma tui`.
  start_process_calls="$(rg -o --no-filename 'startProcess\(' "$WORKBENCH_SOURCES" 2>/dev/null | wc -l | tr -d ' ')"
  if [[ "$start_process_calls" -ne 1 ]]; then
    echo "FAIL: the workbench host must have exactly one bundled luma TUI start path"
    fail=1
  fi
fi
# application → storage is allowed (settings adapters); engine must not open stores directly.
if rg -n 'ClipboardStore::luma_next_default|WordbookStore::luma_next_default|RecordsStore::luma_next_default' \
  crates/luma-application/src/engine.rs crates/luma-application/src/engine/*.rs 2>/dev/null | head -20 | grep .; then
  echo "FAIL: engine must not open stores directly (use injected repositories in compose.rs)"
  fail=1
fi

if rg -n 'ConfigStore::luma_next_default|MacPasteboard::|MacOpenPath' \
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
