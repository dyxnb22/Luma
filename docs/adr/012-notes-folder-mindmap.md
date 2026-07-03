# ADR-012: Notes Folder Tree Mind Map View

## Status

Accepted. Supplements ADR-008 (historical) and ADR-019 by documenting an in-scope visualization that ADR-008's non-goals language could be read as excluding.

Date: 2026-06-22

## Context

`NotesMindMapView` (`Sources/LumaApp/Launcher/NotesMindMapView.swift`) renders the Notes vault folder tree as a spatial layout inside the Route B detail panel. ADR-008 lists "graph view" as an explicit non-goal, referring to wiki-link backlink graphs and Obsidian-style knowledge graphs — not a filesystem tree layout.

Without documentation, this view appears to violate ADR-008's spirit of keeping Notes a minimal markdown index. The view is already shipped and used for orienting within a deep folder hierarchy.

## Decision

The folder-tree mind map is **in scope** for Notes v0.1 detail view:

- Visualizes the same in-memory `NotesTreeIndex` tree already built at warmup.
- Does not parse markdown bodies, wiki-links, tags, or frontmatter.
- Does not add search dimensions beyond what the launcher bar already provides.
- Opens `.md` files in Typora on double-click — same action as the outline view.

### Still out of scope

- Wiki-link backlink / forward-link graphs.
- Note-body semantic layout (clustering by content similarity).
- Any rendering of markdown inside Luma.

## Consequences

- ADR-008 non-goals remain unchanged for wiki graphs and markdown rendering.
- Future graph features still require a superseding ADR.
- `NotesMindMapView` must stay a thin presentation layer over `NotesTreeIndex`; no independent scan or persistence.
