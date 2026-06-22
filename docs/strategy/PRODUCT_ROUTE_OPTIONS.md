# Product Route Options

Status: route comparison index  
Date: 2026-06-22

Luma currently has two documented routes. They are intentionally different and should not be blended accidentally.

## Route A: Launcher Convergence

Document:

- `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`

Shape:

- Small pure launcher.
- No dashboard.
- No in-panel module details.
- Empty query shows usage-backed recent/frequent results.

Best when:

- The priority is speed, reliability, and low maintenance.
- The app should replace Spotlight as the daily hotkey.
- One person is maintaining the product.

Primary tradeoff:

- Gives up the broader dashboard/workbench ambition.

## Route B: Dashboard Widget Single Window

Documents:

- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
- `docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md`

Shape:

- 860 x 540 liquid-glass panel.
- Top search.
- Left running-app sidebar.
- Center widget cards.
- Search results overlay the grid.
- Module details open in the same panel.

Best when:

- The priority is visual richness and a personal command-center feel.
- The user explicitly wants a widget/dashboard first screen.
- Same-panel module detail navigation is required.

Primary tradeoff:

- Higher UI complexity, more state management, and more ways for the launcher hot path to regress.

## Decision Rule

Do not implement both routes at the same time.

If Route A is active:

- Remove dashboard from the launcher panel.
- Keep module details in separate windows.
- Keep the panel smaller and result-focused.

If Route B is active:

- Accept the larger panel and same-window state machine.
- Keep cards to 4-8 maximum.
- Treat liquid glass, sidebar, feature grid, results overlay, and detail container as one integrated surface.

## Current Repo Note

`docs/adr/006-launcher-convergence.md` records Route A as the accepted v1 strategy at the time it was written. If Route B becomes the chosen product direction, add a new ADR superseding ADR-006 rather than silently editing old decisions.

