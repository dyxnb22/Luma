# Notes

## Goal

Local-first Markdown workspace for long-term personal use (ADR-019). Luma indexes, organizes, and opens notes by keyboard; **Typora** edits and renders. Luma is not a note editor.

## Triggers

| Trigger | Behavior |
| --- | --- |
| `n` / `note` / `notes` | Open Notes detail |
| `n <query>` | Fuzzy filename / folder search |
| `n new <title>` | Create note in Inbox and open |
| `n new <template> <title>` | Create from template in Inbox |
| `n daily` / `n today` | Open or create today's daily note |
| `n cap <text>` | Append bullet to today's daily note |
| `n review` / `n review week` | Weekly review note |
| `n doctor` | Health checks (broken links, duplicates, warmup stats) |
| `tag:<name>` / `type:<name>` | Metadata-qualified search |

## Detail view

Frozen IA: `docs/specs/NOTES_DETAIL_CONSTRAINTS.md`.

**Toolbar:** root path; **+ Note** / **+ Folder** (Tree mode); Expand / Collapse; `[Tree | Map]`; settings gear.

**Left chips:** **Today** (quick action — opens daily note, does not stay selected), **Recent**, **Pinned**.

**Right panels:** **Outline** (directory tree only), **Browse** (modified-this-week + groups by frontmatter `type`), **Inbox(n)**.

**Create (detail):** toolbar or `⌘N` / `⌘⇧N`; optional template picker when `_templates/` exists. Default parent folder: selection → note parent → Inbox → root. New notes open in Typora after create.

**In-detail shortcuts:** `⌘1` Today, `⌘2` Recent, `⌘3` Pinned, `⌘L` backlinks, `F2` rename, `⌘⌫` delete.

## Data Model

No Luma-owned note database. Source of truth is a Markdown folder root (`notes.json` stores root path, expansion, recent list, and folder-name conventions).

Index fields (in-memory, warmup + FSEvents):

| Field | Notes |
| --- | --- |
| filename | searchable display title |
| relative path | tree display |
| frontmatter | `title`, `type`, `tags`, `pinned` (read-only subset) |
| FSEvents mtime | change detection |

Body content is not full-text indexed. `n doctor` and backlink discovery read bodies on explicit invocation only.

## Actions

- **Open** (Return / double-click) — Typora; `NSWorkspace.open` fallback
- **Create note / folder** — detail toolbar, context menu, or `n new`
- **Rename / Delete** — Trash via `FileManager.trashItem`
- **Templates** — `_templates/*.md` with `{{title}}`, `{{date}}`, `{{week}}`
- **Append to daily note** — cross-module (`n cap`, Clipboard, Translate)
- **Image Tools** — orphan scan, broken links, migrate to `_assets/`, Typora config check

## Privacy Rules

- Note bodies are not indexed on the query hot path.
- Filenames, folder structure, and minimal frontmatter are indexed in memory.
- Body reads only on explicit actions (`n doctor`, backlinks, template render at create).

## Permissions

No special permissions. File access is scoped to the user-configured root folder.

## Warmup

`warmup` scans the configured root and builds `NotesTreeIndex` and `NotesMetaIndex`. **On-demand** tier — not in global search hot path.

FSEvents keeps indexes current. `handle` is memory-only.

## Implementation Entry

- Module: `Sources/LumaModules/Notes/NotesModule.swift`
- Actions: `Sources/LumaModules/Notes/NotesActions.swift`
- Tree index: `Sources/LumaModules/Notes/NotesTreeIndex.swift`
- Meta index: `Sources/LumaModules/Notes/NotesMetaIndex.swift`
- Templates: `Sources/LumaModules/Notes/NotesTemplateStore.swift`
- Detail view: `Sources/LumaApp/Launcher/NotesDetailView.swift`
- Chip bar: `Sources/LumaApp/Launcher/NotesDetailChipBar.swift`

## Non-Goals

See `docs/adr/019-notes-long-term-markdown-workspace.md` and `docs/NON_GOALS.md` (Notes section). No in-app Markdown editing/rendering, wiki-link graph, full-text body search as a primary surface, multi-vault, sync, or AI features.
