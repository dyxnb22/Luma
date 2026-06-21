# Wordbook

## Goal

Move the native functionality from `/Users/diaoyuxuan/wordbot` into Luma as a modular vocabulary review system.

## Existing Wordbot Capabilities

- SQLite word database at `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- 1,341 words currently present.
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

## Luma MVP Behavior

- Search words from launcher.
- Show due review card.
- Review known/fuzzy/unknown.
- Speak word and example using macOS voice.
- Import existing wordbot database.
- Open word management card.
- Current implementation reads `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3` directly and can search/detect due words.

## Migration Plan

1. Read wordbot SQLite directly.
2. Create Luma tables with compatible columns.
3. Import words and settings.
4. Preserve review scheduling.
5. Keep a backup before migration.

## UI Card

- Shows due count, daily progress, next word, and review buttons.
- Edit button opens word manager.
- Drag position persists in dashboard layout.

## Implementation Entry

- Source module: `Sources/LumaModules/Wordbook/WordbookModule.swift`
- Existing source: `/Users/diaoyuxuan/wordbot`
