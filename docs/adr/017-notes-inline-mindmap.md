# ADR-017: Notes Inline Mind Map

## Status

Accepted (2026-06-22)

## Context

Mind Map was presented as a modal sheet without a dismiss control, trapping users. Product expectation is in-panel toggle between outline and map.

## Decision

- Add `[Tree | Map]` segmented control to `NotesDetailView` toolbar.
- Add **+ Note** / **+ Folder** toolbar buttons (Tree mode only; see `NOTES_DETAIL_CONSTRAINTS.md`).
- Embed `NotesMindMapView` in `mindMapScroll` alongside the outline `scrollView`; toggle visibility.
- Remove `NotesDetailSheets.presentMindMap` and gear menu "Mind Map…".
- Map interactions: single-click select, double-click folder expand/collapse, double-click note opens via Typora.
- Expand/Collapse toolbar buttons visible only in Tree mode; filter applies only to Tree.
- Create toolbar buttons visible only in Tree mode.

## Consequences

- Esc closes the launcher detail (same as Tree) — no sheet trap.
- Large trees: map starts collapsed (root only) to avoid layout cost; user expands folders manually.
