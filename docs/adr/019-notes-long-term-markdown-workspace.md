# ADR-019: Notes as a Long-Term Markdown Workspace

## Status

Accepted (2026-06-23)

Supersedes: `docs/adr/008-notes-markdown-manager.md`

## Context

ADR-008 intentionally reduced Notes to a Markdown file manager so the project could ship a small, safe first version. That decision was correct for the initial boundary.

The current product direction has moved further. Notes is no longer just a file index:

- it is the place where long-lived notes are created
- it is the place where notes are revisited and organized
- it is the place where the local Markdown library is shaped over time
- it is the place where a lightweight personal knowledge workspace can emerge

The problem is not that the scope grew. The problem is that the old ADR no longer describes the system the repository is trying to build.

We need a new boundary that keeps Notes lightweight while allowing the product to mature.

## Decision

Notes is a **local-first Markdown workspace for long-term personal use**.

This means Notes may provide:

- root selection
- tree indexing
- filename and lightweight metadata search
- same-panel folder and note navigation
- note creation, rename, move, and trash flows
- templates for common note kinds
- recent and pinned access patterns
- lightweight topic and review views
- optional on-demand backlink discovery

Notes still must not become:

- a Markdown editor
- a Markdown renderer
- a cloud sync product
- a collaboration product
- a plugin runtime
- a database-first document system
- a graph-first knowledge app

## Explicit scope

The following are in scope for the new Notes direction:

- one configurable root directory
- Markdown files and folders as the canonical store
- a memory-backed tree index
- quick create and tidy-up actions
- templates and conventions for common note kinds
- simple retrieval helpers such as recent, pinned, and topic-oriented views
- optional lightweight metadata such as tags or type hints if they remain human-readable
- on-demand backlinks or related-note discovery if they stay lightweight and non-invasive

## Explicit non-goals

The following remain out of scope:

- full-text body indexing as a primary product surface
- Markdown editing or rendering inside Luma
- transclusion or embedded note rendering
- multi-vault support
- sync and collaboration
- database-style block models
- complex tag hierarchy management
- query-builder style saved databases
- plugin APIs
- AI features
- heavy graph visualization as the core navigation model

## Consequences

Positive:

- Notes can evolve into a durable personal workspace without turning into a second product
- the system remains local-first and portable
- the app can support long-term use without locking the user into a custom data model
- the current launcher-centered architecture still makes sense

Negative:

- Notes is allowed to do more than ADR-008 originally allowed
- this increases implementation responsibility around conventions, templates, and retrieval
- there is a risk of drifting toward complexity if the new boundaries are not enforced

## Implementation implications

If accepted, this ADR implies the following product posture:

- `NotesModule` remains the boundary for note actions and dispatcher behavior
- `NotesTreeIndex` remains the in-memory source for navigation and search
- `NotesRootConfig` remains the lightweight persisted configuration surface
- `NotesDetailView` can evolve into a stronger workspace view, but not into a full editor
- commands for create, move, archive, and revisit should remain explicit and reversible

Detail IA and create flows: `docs/specs/NOTES_DETAIL_CONSTRAINTS.md`  
Feature README: `Features/Notes/README.md`

Format specification: `docs/specs/NOTES_FORMAT.md`

Portability script: `scripts/verify_notes_portability.sh`

## Migration from ADR-008

ADR-008 should no longer be treated as the final statement of Notes scope if this ADR is accepted.

The conceptual change is:

- from "Markdown file manager"
- to "long-term Markdown workspace"

The practical change is:

- from read-only indexing and launching
- to lightweight workspace support for capture, organization, retrieval, and archival

The boundary that remains unchanged is:

- Luma does not own the note content format
- the user remains in control of the files
- the editor stays outside Luma
- the product stays lightweight

## Notes

This ADR intentionally keeps Notes smaller than Obsidian and simpler than Notion.

It allows Luma to support a meaningful personal note workflow without inheriting the maintenance burden of a full document platform.

