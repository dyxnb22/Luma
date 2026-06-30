# Notes

## Goal

Markdown file index and Typora launcher. Luma lets you search and open notes by filename without leaving the keyboard. It is not a note editor.

## Triggers

- `n` / `note` / `notes` — open Notes detail (tree view)
- `n <query>` — search note filenames and folder names
- `n daily` / `n today` — jump to or create today's daily note
- `n review` / `n review week` — open weekly review note
- `n doctor` — run health checks (broken links, duplicate names, warmup stats)
- `n new <name>` — create a new note file

## Data Model

No Luma-owned database. Source of truth is a Markdown folder root configured in Settings (stored in `notes.json`).

Index fields (in-memory only):

| Field | Notes |
|-------|-------|
| filename | searchable, used as display title |
| relative path | for tree display |
| FSEvents mtime | for change detection |

Frontmatter is parsed for `daily` and `template` markers only. Body content is never read or indexed.

## Actions

- **Open** (Return) — opens in Typora; falls back to `NSWorkspace.open` if Typora is not installed
- **Create note** — writes a new `.md` file at the configured root
- **Rename / Delete** — folder + note management from the detail tree view
- **Append to daily note** — cross-module action available from Clipboard and Translate

## Privacy Rules

- Note bodies are never read or stored.
- Only filenames and folder structure are indexed.
- `n doctor` reads note bodies for wiki-link validation but only on explicit invocation — never on the query hot path.

## Permissions

No special permissions required. File access is scoped to the user-configured root folder.

## Warmup

`warmup` scans the configured root via `FSEventsService` and builds `NotesTreeIndex` and `NotesMetaIndex`. This is a filesystem scan — **Phase 2** only. Never add to `BuiltInModules.fastModuleIDs`.

FSEvents watcher keeps the index current after warmup. `handle` is memory-only.

Warmup duration is tracked in `NotesModule.lastWarmupMilliseconds()` and surfaced in `n doctor`.

## Implementation Entry

- Module: `Sources/LumaModules/Notes/NotesModule.swift`
- Tree index: `Sources/LumaModules/Notes/NotesTreeIndex.swift`
- Meta index: `Sources/LumaModules/Notes/NotesMetaIndex.swift`
- Health checks: `Sources/LumaModules/Notes/NotesDoctor.swift`
- Root config: `Sources/LumaModules/Notes/NotesRootConfig.swift`
- Detail view: `Sources/LumaApp/Launcher/NotesDetailView.swift`

## Non-Goals

See `docs/NON_GOALS.md` — Notes section. Backlinks, graph view, full-text search, Markdown rendering, tags, frontmatter generation, multi-vault, and AI features are all explicitly out of scope.
