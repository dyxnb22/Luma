# ADR-007: Dashboard Widget Single Window

## Status

Superseded by ADR-023. Historical record of Route B implementation.

This ADR is retained for implementation archaeology only. Do not use it as guidance for current home navigation, keyboard semantics, or launcher layout.

## Context

Luma currently chooses a single-window shape where launcher, dashboard, and module detail views live in the same panel:

> Command+Space -> liquid-glass panel -> search / sidebar / main content area.

This route is more complex than a pure launcher, but it matches the current product goal: a personal command center with widget-style feature cards, search results overlay, and same-panel module detail navigation.

ADR-006 recorded Route A (pure launcher convergence) as the accepted v1 strategy. Route B is now the active UI implementation direction. This ADR supersedes ADR-006 for UI decisions without erasing the historical record of Route A.

## Decision

Route B becomes the current UI implementation route:

- Fixed panel size: 860 x 540 pt.
- Top search bar, always visible.
- Left Open Apps sidebar, always visible.
- Main content area switches among three states:
  - feature grid (empty query)
  - search results (non-empty query)
  - module detail (card selection)
- Visual direction: iOS/macOS liquid-glass stack (blur, top highlight, inner border, panel shadow).
- Default hotkey remains Command+Space.

Do not blend Route A pure-launcher rules with Route B dashboard rules in the same implementation pass.

## Consequences

Positive:

- Clear product identity as a command-center launcher.
- Same-panel module detail avoids extra windows for core workflows.
- Visual richness aligned with widget/dashboard ambition.

Negative:

- Higher UI state complexity than Route A.
- Feature card count must stay limited; recommend 4-8 cards maximum.
- Hot-path performance discipline is mandatory; panel size and state machine add regression risk.
- Docs, tests, and `.cursor/rules.md` must track Route B as active; Route A remains reference only until explicitly revived.

## Implementation Notes

- Add feature cards, sidebar frecency, module detail views, and search result UI in later phases per `docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md`.
- Do not delete experimental module code while stabilizing the Route B shell.
- Keep LatencyHUD and other debug UI out of the production panel.
