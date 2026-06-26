# ADR-024: Current Project Context

Status: accepted  
Date: 2026-06-26

## Context

IDE window title parsing (`IDEWindowTitle`) existed for Open Apps but was not connected to project lookup or snippet/command templates.

## Decision

Add `CurrentProjectService` (actor, 1.5s TTL) that reads the frontmost IDE window via AX and matches labels through `ProjectIndex.matchByLabel`.

Expose context on the launcher home row and in `SnippetExpansionContext` / `commands.json` template expansion.

## Consequences

- Requires Accessibility permission for IDE context features.
- Project path matching depends on `projects.json` records (manual or scanned).
