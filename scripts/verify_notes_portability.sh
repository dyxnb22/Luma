#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Notes portability verification =="
echo "Runs Swift tests that rebuild a vault from Markdown files with an empty notes.json cache."
echo "Spec: docs/ENGINEERING.md and docs/MODULES.md"
echo ""

swift test --filter NotesPortability 2>&1

echo ""
echo "== Manual spot-check (optional) =="
echo "1. Copy your vault to a temp folder"
echo "2. rm ~/Library/Application\\ Support/Luma/notes.json"
echo "3. Open Luma → Notes → set root to the copy"
echo "4. Confirm tree, search, and n new still work within ~30s"
