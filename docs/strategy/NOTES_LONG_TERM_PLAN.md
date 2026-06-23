# Notes Long-Term Plan

Status: strategy draft  
Date: 2026-06-23  
Companion ADR draft: `docs/adr/019-notes-long-term-markdown-workspace.md`

## 0. Why this plan exists

Notes has already moved beyond a thin file-manager idea. The current implementation is starting to carry the weight of a real personal knowledge workspace: root selection, tree index, recent notes, same-panel details, image tools, and exploration of backlinks and mind-map style navigation.

That is not a problem by itself. The problem is when the product expands without a clear long-term shape.

This plan defines that shape:

- Notes is a local-first Markdown workspace for long-term personal use.
- It should feel lighter than Obsidian and more portable than Notion.
- It should support stable daily capture, later review, and low-friction retrieval.
- It must remain simple enough that one person can maintain it for years.

The core rule is unchanged:

- Filesystem is the source of truth.
- Luma indexes and orchestrates.
- Typora or another editor owns editing and rendering.

## 1. North Star

**Luma Notes is my lowest-friction entry point into a long-lived Markdown knowledge base.**

It is not:

- an editor
- a database product
- a collaboration suite
- a graph-first app

It is:

- a fast index
- a lightweight organizer
- a safe entry point for creation and retrieval
- a stable local workspace built for years of incremental use

## 2. What Notes should become

Notes should gradually evolve from "file browser" to "learning workspace" without becoming heavy.

The target shape is:

- fast capture
- stable folder and file conventions
- optional lightweight structure
- practical retrieval
- same-panel navigation
- minimal friction for weekly review and archiving

The target feeling is:

- I can open Notes quickly.
- I can create a new note in a few keystrokes.
- I can find old notes by filename, path, title, and light metadata.
- I can keep the library tidy without a lot of ceremony.
- I can move notes around and still trust that the system remains readable.

## 3. Design principles

1. **Filesystem first**
   - The Markdown files and folders are the canonical data model.
   - This is what makes the system portable and future-proof.

2. **Index, not store**
   - Luma may cache derived data, but it should never require a database to recover the user's content.
   - If the cache disappears, the knowledge base still survives.

3. **Editor boundary is sacred**
   - Notes should not become a Markdown editor or renderer.
   - Editing belongs to Typora or another chosen editor.

4. **Low friction beats expressive power**
   - If a feature makes capture or retrieval slower, it is suspect.
   - Notes should optimize for repeated use, not occasional wow.

5. **Light structure over heavy schemas**
   - Use conventions, templates, and simple metadata.
   - Avoid turning note organization into a database design exercise.

6. **Portable by default**
   - Content should remain understandable outside Luma.
   - Any metadata should be simple enough to survive migration.

7. **Human control first**
   - Automatic organization should be suggestive, reversible, and visible.
   - Silent rewriting of files should be avoided.

8. **Search before hierarchy, hierarchy before graph**
   - Search and folder structure should do most of the work.
   - Graph-style views are optional and should never be the primary navigation model.

9. **Single-root simplicity**
   - One main root is enough for the first long-term version.
   - Multi-root introduces ambiguity and maintenance cost.

10. **Stable conventions beat clever features**
    - A few naming conventions should outlive many UI experiments.
    - The system should remain useful even if the UI changes later.

11. **Capture, revisit, archive**
    - A note system is only useful if capture is easy, revisit is natural, and archive is safe.
    - All three must be designed together.

12. **Lightweight is a feature**
    - Not doing something is a valid design choice.
    - If a feature only adds complexity, it needs a strong reason to exist.

## 4. What to borrow from Obsidian and Notion

| Source | Worth borrowing | Lightweight Luma version | Do not copy | Why not |
| --- | --- | --- | --- | --- |
| Obsidian | Markdown as the primary format | Plain `.md` files in a normal folder tree | Custom storage format | Reduces portability |
| Obsidian | Wiki-linking and local linking | Optional `[[Title]]` style linking and jump-to-note behavior | Heavy embedded rendering | Forces a renderer stack |
| Obsidian | Local-first ownership | Entirely local roots and local discovery | Sync-centric product model | Adds cloud dependence |
| Obsidian | Fast folder-first navigation | Tree index, recent notes, search, jump commands | Plugin ecosystem | Expands scope indefinitely |
| Notion | Light structure through templates | Templates for note kinds like daily/review/meeting/topic | Full database properties | Pulls the product toward schema-first design |
| Notion | Curated views | Simple filtered lists and saved views for common note groups | Complex query builder | Becomes a second product |
| Notion | Clear organization affordances | Tags, pinned notes, and folder conventions | Block database model | Not portable and too heavy |
| Notion | Easy reorganization | Rename, move, archive, and simple batch flows | Rich collaboration features | Not relevant for one-user local use |

## 5. Information architecture

Notes should primarily hold content that benefits from being written, searched, linked, and revisited over time.

Good fits:

- learning notes
- reading notes
- meeting notes
- project notes
- daily notes
- review notes
- reference notes
- ideas and drafts
- task context and decision logs
- reusable templates

Also acceptable:

- lightweight indexes
- topic pages
- archive pages
- note collections

Better handled elsewhere:

- one-off clipboard content
- short-lived action items
- secret values
- snippet-like reusable commands
- structured vocabulary or flashcard systems

Poor fits:

- highly sensitive documents
- transient reminders
- media libraries
- content that needs rich collaboration or permissions

The rule of thumb is simple:

- If it is meant to be remembered, linked, and reviewed, Notes is a good fit.
- If it is meant to be executed, copied, scheduled, or stored securely, another module should own it.

## 6. Long-term usage loop

Notes should support seven repeating activities.

### 6.1 Capture

Goal:

- create a note in seconds

Implementation shape:

- quick create commands
- root-aware default folders
- templates for common note types
- inbox-like fallback for uncategorized notes

Human role:

- write the content

System role:

- create the file
- apply the template
- open the note immediately

### 6.2 Organize

Goal:

- move content into stable homes without making organization tedious

Implementation shape:

- rename
- move
- archive
- folder-level context menus
- simple pinned or starred states

Human role:

- decide the right home

System role:

- keep the tree consistent
- preserve references when possible

### 6.3 Link

Goal:

- connect related notes with low effort

Implementation shape:

- wiki-link support by convention
- jump-to-note behavior
- optional backlinks surfaced on demand

Human role:

- decide what should connect

System role:

- resolve and surface links

### 6.4 Revisit

Goal:

- surface useful notes again without turning the app into a recommendation engine

Implementation shape:

- recent notes
- pinned notes
- topic pages
- lightweight saved views

Human role:

- choose what to revisit

System role:

- keep recent and pinned content easy to reach

### 6.5 Reuse

Goal:

- make old notes reusable as starting points

Implementation shape:

- templates
- duplicate note
- copy note path
- open in editor

Human role:

- choose reuse patterns

System role:

- make reuse commands fast

### 6.6 Review

Goal:

- support periodic self-review

Implementation shape:

- daily note
- weekly review
- monthly review
- archive review

Human role:

- write the reflection

System role:

- expose the review entry points

### 6.7 Archive

Goal:

- keep the active workspace clean without losing history

Implementation shape:

- archive command
- archive folders by year
- read-only or reduced-visibility archive views

Human role:

- decide what is done

System role:

- move content safely
- preserve accessibility

## 7. Capability layers

### 7.1 Base layer

Must have:

- root selection
- tree index
- search by filename, folder name, and light metadata
- open note in external editor
- create note and folder
- rename
- delete to Trash
- recent notes
- folder expansion state

Can have:

- reveal in Finder
- copy path
- keyboard shortcuts for common actions

Too early:

- full-body indexing
- renderer features
- heavy query language
- multi-root support

### 7.2 Learning layer

Must have:

- templates
- note kinds or lightweight types
- pinned or favorite notes
- daily note
- review note
- topic note

Can have:

- basic tags
- topic pages
- simple saved collections

Too early:

- tag management UI
- tag hierarchy
- database-style property editing
- block references

### 7.3 Enhancement layer

Can have if justified:

- backlinks on demand
- related notes list
- archive shortcuts
- simple overview pages
- filtered views by folder or lightweight metadata

Too early:

- graph-first navigation
- live backlink maintenance
- full-text body search
- rich transclusion
- embedded note rendering

### 7.4 Long-term layer

Must have:

- exportability
- readable file formats
- backup friendliness
- recovery after config loss
- compatibility with simple editors

Can have:

- migration helpers
- integrity checks
- archive report

Too early:

- application-level sync
- encrypted content store
- version control replacement

## 8. What should be true about the UI

Notes should be optimized for:

- fast capture from the launcher
- low-friction tree navigation
- a same-panel detail experience
- simple empty states
- clear context menus
- predictable keyboard navigation

Notes should not be optimized for:

- showing off complexity
- replacing the editor
- replacing the filesystem
- looking like a document app with a huge feature surface

The UI should make it easy to answer four questions:

- What did I write recently?
- What is worth opening now?
- How do I create a new note quickly?
- How do I keep the library tidy?

## 9. Product roadmap

### Phase 1: Stabilize the boundary

Goal:

- align implementation with a clear Notes identity

Focus:

- remove hot-path I/O from launcher query handling
- keep the tree index in memory
- make creation and opening boring and reliable
- keep the editor boundary intact

Risk:

- complexity already introduced by exploratory features

Success:

- Notes feels fast, simple, and trustworthy

### Phase 2: Capture and organization

Goal:

- make note creation and cleanup frictionless

Focus:

- quick create commands
- templates
- rename/move/delete
- recent notes
- archive flow
- folder and note conventions

Risk:

- over-designing metadata or forcing structure

Success:

- creating and filing notes feels natural

### Phase 3: Retrieval and revisit

Goal:

- make the library useful again after time has passed

Focus:

- pinned notes
- topic pages
- lightweight filtered views
- backlinks on demand
- simple archive browsing

Risk:

- drifting toward graph-first or database-first behavior

Success:

- old notes are easy to find, and useful content reappears naturally

### Phase 4: Long-term durability

Goal:

- make the system resilient for years

Focus:

- exports
- backup hygiene
- format stability
- migration helpers
- recovery from broken config or moved folders

Risk:

- adding maintenance features that only serve edge cases

Success:

- the library remains portable and recoverable

## 10. MVP for the current repository

The best short-term MVP is small and strict:

1. Make the Notes entry point reliably fast.
2. Keep the tree index as the main interaction model.
3. Add quick create and clean delete/move flows.
4. Add templates for daily/review/topic note types.
5. Add recent/pinned/topic-style retrieval that stays lightweight.

Why these first:

- they improve daily usefulness immediately
- they reinforce long-term organization without heavy systems
- they fit the current AppKit and module architecture

Why not larger features first:

- full graph and body indexing create maintenance load
- complex property systems invite Notion-like weight
- editor replacement would derail the project

## 11. Anti-goals

Do not build:

- a cloud-first note service
- a collaboration platform
- a database-first document app
- a full Markdown editor
- a rendering engine inside Luma
- a plugin marketplace
- a block-based lock-in model
- a graph-first product
- a semantic search product
- a heavy tag-management system
- a complex query builder
- multi-root vault management
- comment threads or permissions
- hidden automatic file rewriting
- anything that makes the app harder to trust

## 12. Success metrics

The plan succeeds if:

- capture feels fast enough to use every day
- retrieval is easier than manual folder hunting
- the library stays portable and readable outside Luma
- the app remains simple enough to maintain
- users do not need to think about the system to benefit from it
- the workspace grows without becoming chaotic
- the UI remains lightweight under real use

Quantitative signs:

- fewer abandoned notes in Inbox
- faster time from command to note creation
- fewer searches that fail because the note was hard to retrieve
- lower maintenance burden for the note library

## 13. Implementation notes

The current codebase already suggests a good split:

- `NotesModule` should remain the module boundary and action dispatcher.
- `NotesTreeIndex` should own the in-memory tree and search primitives.
- `NotesRootConfig` should own root and expansion state.
- `NotesDetailView` should own the same-panel file navigation experience.
- note creation, rename, and deletion should stay explicit and local.

If a feature makes Notes feel heavier than a file manager plus organizer, it needs a very strong reason.

The long-term direction is not to expand indiscriminately.

The long-term direction is to make the few most important note workflows excellent.

