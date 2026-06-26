# ADR-026: Snippet Variables Expanded

Status: accepted  
Date: 2026-06-26

## Context

`SnippetVariableExpander` only supported `{{date}}`, `{{clipboard}}`, and a misleading `{{cursor}}` (empty replacement).

## Decision

Introduce `SnippetExpansionContext` and add: `{{uuid}}`, `{{timestamp}}`, `{{date:format}}`, `{{selection}}`, `{{project}}`, `{{project_path}}`, `{{file}}`, `{{filename}}`.

Rename misleading cursor semantics to `{{caret}}` (paste leaves cursor at end). Keep `{{cursor}}` as alias for compatibility.

## Consequences

- Expansion is no longer a pure function of clipboard only; callers must supply context at paste time.
- True in-text caret placement remains out of scope.
