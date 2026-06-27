# Wordbook

## Goal

Provide a same-panel vocabulary review workflow inside Luma while preserving the existing Wordbot data and review logic where practical.

## Migration Context

- Legacy source: `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- The migration source remains useful for compatibility checks and import behavior.
- Luma should keep review behavior stable rather than reinvent the learning model casually.

## Existing Wordbot Capabilities

- SQLite word database at `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- Fields: term, phonetic, meaning, example, category, familiarity, review stage/count, wrong count, next review time, last review time.
- British English pronunciation preference.
- CSV and Markdown/table/text import.
- Ebbinghaus-style 9-stage review intervals:
  - 5 minutes.
  - 30 minutes.
  - 12 hours.
  - 1 day.
  - 2 days.
  - 4 days.
  - 7 days.
  - 15 days.
  - 30 days.
- Review responses: known, fuzzy, unknown.
- Daily goal and progress.

## Active Behavior (Route C)

- `word` or `word review` opens the same-panel review flow.
- `word <query>` searches vocabulary from the launcher.
- Review uses the current three-grade flow and keyboard shortcuts.
- The session survives panel hide/show and returns to the same word when appropriate.
- Speak word and example using macOS voice.
- Manage view, CSV import, and settings live in-panel.
- Current implementation uses the Luma-owned Wordbook store after migration, with legacy Wordbot data retained as source material.

## Migration Plan

1. Read Wordbot SQLite.
2. Import words and settings into Luma storage.
3. Preserve review scheduling and due counts.
4. Keep a backup before migration.
5. Verify same-panel review after import.

## Implementation Entry

- Source module: `Sources/LumaModules/Wordbook/WordbookModule.swift`
- Existing source: `/Users/diaoyuxuan/wordbot`
