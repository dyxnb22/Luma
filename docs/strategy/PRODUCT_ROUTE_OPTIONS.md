# Product Route Options

Status: route comparison index  
Date: 2026-06-25 (updated for Route C)

Luma has three documented routes. They are intentionally different and should not be blended accidentally.

## Route C: Command-First Unified List (active)

Document:

- `docs/adr/023-command-first-unified-list.md`

Shape:

- ~700 × 480 pt single-column list.
- Empty query: Open Apps, Suggested, Recent sections.
- Non-empty query: flat ranked results; Return runs primary action.
- Tab / ⌘K opens Action Panel; module details stay in the same panel.
- **No dashboard feature-card grid** and **no permanent sidebar**.

Best when:

- The priority is one-keyboard-step command execution.
- Modules are discovered via triggers, recents, and suggestions.
- The hot path must stay fast and visually simple.

Primary tradeoff:

- Less visible module discovery than a dashboard grid.

**Current repo:** Route C is active per ADR-023. It supersedes ADR-007 for UI implementation.

## Route A: Launcher Convergence (historical)

Document:

- `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`
- `docs/adr/006-launcher-convergence.md`

Shape:

- Small pure launcher.
- No dashboard.
- No in-panel module details.
- Empty query shows usage-backed recent/frequent results.

## Route B: Dashboard Widget Single Window (historical)

Documents:

- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md` (marked historical)
- `docs/adr/007-dashboard-widget-single-window.md` (superseded by ADR-023)

Shape:

- 860 × 540 liquid-glass panel.
- Top search.
- Left running-app sidebar.
- Center widget cards.
- Search results overlay the grid.
- Module details open in the same panel.

## Decision Rule

Do not implement multiple routes at the same time.

If Route C is active (current):

- Keep a single list column; home sections only on empty query.
- Surface modules via `ResultItem` rows and explicit triggers.
- Do not reintroduce dashboard cards or a permanent sidebar as home entry points.

If reviving Route A or Route B:

- Add a new ADR superseding ADR-023 rather than silently editing old decisions.

## Current Repo Note

`docs/adr/023-command-first-unified-list.md` records Route C as the accepted UI direction (2026-06-25). ADR-007 and ADR-006 remain historical records.
