# ADR-024: Current Project Context

Status: accepted (home row superseded by ADR-032)
Date: 2026-06-26

> **2026-07 update:** Frozen empty-query home (ADR-032) no longer shows a project context row. Current project context lives in `proj` / project workspace detail and template expansion (`SnippetExpansionContext`, `commands.json`). This ADR remains authoritative for matching and service shape only.

## Context

IDE window title parsing (`IDEWindowTitle`) existed for Open Apps but was not connected to project lookup or snippet/command templates.

## Decision

Add `CurrentProjectService` (actor, 1.5s TTL) that reads the frontmost IDE window via AX and matches labels through `ProjectIndex.matchByLabel`.

Expose context in `SnippetExpansionContext` / `commands.json` template expansion and the project workspace detail (`proj` commands). Do not restore a home-row project context entry without a new ADR.

## Consequences

- Requires Accessibility permission for IDE context features.
- Project path matching depends on `projects.json` records (manual or scanned).
