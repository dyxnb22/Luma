# ADR-008: Notes Reduced to Markdown File Manager

## Status

Accepted, supersedes the Notes scope previously documented in `Features/NotesGraph/README.md`.

Date: 2026-06-22

## Context

The original Notes scope (`Features/NotesGraph/README.md`) promised an Obsidian-like local vault with wiki-link backlinks, tags, a graph index in SQLite, an Inbox count, orphan-note count, and dashboard card metadata. The existing implementation reflects that ambition only as scaffolding:

- `Sources/LumaModules/Notes/NotesModule.swift` (78 lines)
- `Sources/LumaModules/Notes/NotesVaultStore.swift` (62 lines)
- `Sources/LumaModules/Notes/NotesGraphIndexer.swift` (49 lines)

That scope has two problems that block shipping:

1. **It is too large for a single-maintainer v1.** Backlinks, graph, tags, frontmatter parsing, and a SQLite store are weeks of work each and each invite further scope (orphan reports, dataview-style queries, plugin hooks).
2. **The current implementation already violates the module contract.** `NotesModule.handle()` calls `NotesVaultStore.scan()` per query; `scan()` walks the vault on disk and reads every `.md` file into memory. The contract requires `handle` to answer from memory only and to keep disk I/O out of the hot path (`docs/specs/MODULE_CONTRACT.md`, `docs/specs/PERFORMANCE.md`).

We also explored adding two new features:

- **Notion-style page embedding (transclusion).** Markdown has no native concept of embedding; Obsidian's `![[Note]]` is a renderer extension, and Typora does not render it. Supporting it would force Luma to ship its own Markdown renderer, which contradicts the "Typora owns editing/rendering" boundary and triggers a multi-quarter scope explosion (MathJax, Mermaid, code highlighting, edit cursor, undo stack). Cheaper outcomes (open a related note in Typora, list `[[wiki]]` mentions) deliver the underlying user value without rendering.
- **Unified image management.** Typora already handles most of this via its "Image" preferences. The residual pain (orphan images, broken links, scattered locations, absolute paths) maps to a small one-shot utility, not an ongoing system.

## Decision

Notes v0.1 is **a Markdown file index and Typora launcher**, nothing more.

Concretely:

- One configurable local root directory. No multi-vault.
- Warmup builds an in-memory tree of folders and `.md` files; `handle` answers only from that tree.
- FSEvents keeps the tree current; there is no manual refresh button.
- Same-panel detail view (Route B, per ADR-007) renders the tree in an `NSOutlineView`.
- Double-click or Return on a `.md` node opens the file in Typora (`NSWorkspace.open` fallback).
- Top launcher search bar drives filename matching across the tree; this is the core Luma value-add and the reason Notes remains a module rather than a standalone app.
- Create note / create folder / rename / delete (`.md` files and empty folders only). Deletes go to Trash via `FileManager.trashItem`, never `unlink`.
- A small "Image Tools" panel inside the detail view provides four one-shot commands: orphan scan, broken-link scan, migrate-to-`_assets/`, Typora config check.

The module's display name changes from `Notes Graph` to `Notes`.

### Explicit Non-Goals

The following are **not** in v0.1 and are not deferred features awaiting prioritisation; they are out of scope by design:

- Page embedding / transclusion / `![[Note]]` rendering.
- Wiki-link backlinks, forward-link graphs, graph view, orphan dashboards beyond the image utility.
- Tags, frontmatter parsing, frontmatter generation, templates.
- Markdown rendering of any kind. Markdown editing of any kind.
- Full-text search of note bodies. Filename and folder name search only.
- AI summarisation, AI Q&A, semantic search.
- Multi-vault, vault sync, vault import/export, version history.
- Image insertion / paste interception / image gallery / image compression / format conversion.
- Drag-to-move tree nodes.
- Inbox count, recent count, orphan count surfaced on the dashboard card.

Reviving any of these requires a superseding ADR.

## Consequences

Positive:

- Scope fits one maintainer and ships in days, not quarters.
- Boundary with Typora is crisp: Luma indexes and dispatches; Typora renders and edits.
- Module conforms to `MODULE_CONTRACT.md`: `handle` becomes pure memory lookup.
- FSEvents wiring forces `LumaServices/FileSystem/FSEventsService.swift` to become a real service (currently a stub), which benefits any future module that needs file watching.

Negative:

- Users coming from Obsidian will find the feature surface thin. This is intentional; if they want Obsidian they can run Obsidian alongside.
- Filename-only search misses notes whose useful keywords live in the body. Acceptable for v0.1; revisit only with evidence (search misses logged during dogfooding).
- The reduced surface still consumes one of the 4-8 feature card slots in Route B. If launcher-hit usage does not materialise during the first month of dogfooding, the next ADR should remove Notes from the card grid entirely.

## Implementation Notes

- Implementation phases are defined in `docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md`.
- `Features/NotesGraph/README.md` is marked superseded by this ADR and should not be treated as a source of truth.
- Existing files to delete: `Sources/LumaModules/Notes/NotesGraphIndexer.swift`, `Sources/LumaModules/Notes/NotesVaultStore.swift`. Replace with `NotesTreeIndex.swift`, `NotesActions.swift`, `NotesRootConfig.swift`.
- Persistence file: `~/Library/Application Support/Luma/notes.json` (schema: `{ "root": "...", "expandedFolders": [...] }`).
- Update `docs/specs/PERFORMANCE.md` to include the Notes-specific `handle` budget (p95 ≤ 30 ms warm).
- Update `docs/NON_GOALS.md` if it does not already capture the non-goals above.
