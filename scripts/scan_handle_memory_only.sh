#!/usr/bin/env bash
# P2.3 — static proxy: P0 module handle() must not call forbidden hot-path APIs.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODULES=(
  "Sources/LumaModules/Apps/AppsModule.swift"
  "Sources/LumaModules/Clipboard/ClipboardModule.swift"
  "Sources/LumaModules/Notes/NotesModule.swift"
)
FORBIDDEN=(
  "await accessibility"
  "accessibility.isTrusted"
  "CGWindowListCopyWindowInfo"
  "NSWorkspace.shared"
  "URLSession"
)

fail=0
for rel in "${MODULES[@]}"; do
  path="$ROOT/$rel"
  section=$(python3 - "$path" <<'PY'
import sys, re
text = open(sys.argv[1]).read()
m = re.search(r"public func handle\([^)]*\)[^{]*\{", text)
if not m:
    sys.exit(1)
start = m.start()
depth = 0
i = m.end() - 1
while i < len(text):
    c = text[i]
    if c == "{":
        depth += 1
    elif c == "}":
        depth -= 1
        if depth == 0:
            print(text[start:i+1])
            break
    i += 1
PY
)
  for token in "${FORBIDDEN[@]}"; do
    if echo "$section" | grep -q "$token"; then
      echo "FAIL $rel: handle() contains forbidden token: $token" >&2
      fail=1
    fi
  done
done

if [[ $fail -ne 0 ]]; then
  exit 1
fi
echo "scan_handle_memory_only: OK (${#MODULES[@]} P0 modules)"
