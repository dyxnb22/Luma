# Notes Format Specification

Status: active  
Schema version: 1 (see `NotesRootConfig.schemaVersion`)  
Companion ADR: `docs/adr/019-notes-long-term-markdown-workspace.md`  
Detail IA: `docs/specs/NOTES_DETAIL_CONSTRAINTS.md`  
Feature README: `Features/Notes/README.md`

## Purpose

This document defines the on-disk layout Luma Notes expects. The Markdown files and folders are the canonical store. Luma's `notes.json` is a disposable index cache.

**Compatibility promise:** fields documented here remain readable for at least three years without migration scripts. New optional fields may be added; existing fields are never repurposed.

## Canonical store

| Layer | Location | Required |
| --- | --- | --- |
| Note bodies | `<root>/**/*.md` | Yes |
| Folder structure | Normal directories under `<root>` | Yes |
| Luma cache | `~/Library/Application Support/Luma/notes.json` | No |

Deleting `notes.json` must not lose note content. Luma rebuilds tree, metadata index, and recent list from the filesystem on next launch (target: ≤ 30 seconds for 1000 notes on a warm machine).

## `notes.json` schema v1

```json
{
  "root": "/path/to/vault",
  "expandedFolders": ["..."],
  "recent": ["/path/to/note.md"],
  "inboxFolderName": "Inbox",
  "dailyFolderName": "Daily",
  "templatesFolderName": "_templates",
  "reviewsFolderName": "Reviews"
}
```

| Field | Default | Purpose |
| --- | --- | --- |
| `root` | — | Absolute path to vault root |
| `expandedFolders` | `[]` | Outline expansion state |
| `recent` | `[]` | Last opened note paths (max 8) |
| `inboxFolderName` | `Inbox` | Quick-capture target folder |
| `dailyFolderName` | `Daily` | Daily note folder |
| `templatesFolderName` | `_templates` | Template library |
| `reviewsFolderName` | `Reviews` | Weekly/monthly review notes |

Unknown JSON keys are ignored on read.

## Recommended folder conventions

| Purpose | Path |
| --- | --- |
| Inbox / quick capture | `Inbox/` |
| Daily notes | `Daily/YYYY-MM-DD.md` |
| Templates | `_templates/*.md` |
| Weekly reviews | `Reviews/<year>/<YYYY-Www>.md` |
| Long-term topics | `Topics/<Subject>/` |
| Reading notes | `Reading/` |
| Projects | `Projects/<name>/` |
| Archive | `Archive/<year>/` |

These are conventions, not requirements. Luma create commands default parent folder to **selected folder → parent of note → Inbox → root** (detail UI); launcher `n new` always targets Inbox.

## Frontmatter (read-only in Luma)

Luma parses a minimal YAML subset at index time. Luma does not write frontmatter except via template prefill at create time.

Supported keys:

| Key | Type | Example |
| --- | --- | --- |
| `title` | string | `title: My Note` |
| `type` | string | `type: reading` |
| `tags` | string or list | `tags: swift` or `tags: [swift, notes]` |
| `pinned` | bool | `pinned: true` |

Search qualifiers: `tag:<name>`, `type:<name>`.

No Luma-private fields. Use standard names so Obsidian, Typora, and other tools remain compatible.

## Wiki links

- Syntax: `[[Note Title]]`
- Resolution: filename match (case-insensitive), not path-based
- Luma does not render or embed notes; editors own rendering

## Templates

Templates live in `<root>/_templates/<name>.md`. Variables at create time:

| Variable | Replacement |
| --- | --- |
| `{{title}}` | Note title |
| `{{date}}` | ISO date `YYYY-MM-DD` |
| `{{date\|medium}}` | Locale medium date |
| `{{week}}` | ISO week `YYYY-Www` |

## Health check

Run `n doctor` in the launcher to surface:

- Unclosed frontmatter blocks
- Duplicate note names (case-insensitive)
- Unresolved `[[wiki]]` links
- Vault stats (note count, last warmup time)

## Portability verification

```bash
./scripts/verify_notes_portability.sh
```

This runs automated tests that simulate an empty `notes.json` and verify tree recovery from Markdown files alone.

## Non-goals

See `docs/adr/019-notes-long-term-markdown-workspace.md` non-goals and `docs/NON_GOALS.md`. Notably: no proprietary note format, no SQLite vault, no required cloud sync.
