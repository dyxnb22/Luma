# Notes Graph

## Goal

Manage Markdown notes in an Obsidian-like local vault while keeping Typora as the normal reader/editor. Luma provides capture, search, backlinks, tree browsing, and graph metadata.

## File Model

- Notes are plain `.md` files.
- Default vault path is user-configurable.
- Luma does not own the editor.
- Open/edit action uses Typora.

## MVP Behavior

- Create note.
- Search title/content.
- Open note in Typora.
- Show folder tree.
- Track wiki links: `[[Note Title]]`.
- Track Markdown links.
- Maintain graph index in SQLite for fast search and backlinks.
- Current implementation scans Markdown files from the local vault and opens files in Typora when installed.

## Graph Model

- Node: note file.
- Edge: link, backlink, tag, folder relation.
- Tags from `#tag` and optional frontmatter.

## UI Card

- Shows recent note, inbox count, and orphan note count.
- Edit button opens vault settings.
- Drag position persists in dashboard layout.

## Implementation Entry

- Source module: `Sources/LumaModules/Notes/NotesModule.swift`
- Future store: `module_notes_nodes`, `module_notes_edges`
