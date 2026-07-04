# ADR-007: Dashboard Widget Single Window

## Status

Superseded by ADR-023. Historical record of Route B implementation.

This ADR is retained for implementation archaeology only. Do not use it as guidance for current home navigation, keyboard semantics, or launcher layout.

## Context

At the time, Luma chose a single-window shape where launcher, dashboard, and module detail views lived in the same panel:

> Command+Space -> liquid-glass panel -> search / sidebar / main content area.

This route was more complex than a pure launcher, but it matched that phase's product goal: a personal command center with widget-style feature cards, search results overlay, and same-panel module detail navigation.

ADR-006 recorded Route A (pure launcher convergence) as the accepted v1 strategy at that time. Route B then became the active UI implementation direction. ADR-023 later superseded Route B for current UI decisions without erasing the historical record.

## Decision

Route B became the implementation route for this historical phase:

- Fixed panel size: 860 x 540 pt.
- Top search bar, always visible.
- Left Open Apps sidebar, always visible.
- Main content area switches among three states:
  - feature grid (empty query)
  - search results (non-empty query)
  - module detail (card selection)
- Visual direction: iOS/macOS liquid-glass stack (blur, top highlight, inner border, panel shadow).
- Default hotkey remains Command+Space.

Do not blend Route A pure-launcher rules or Route B dashboard rules into current Route C implementation work.

## Consequences

Positive:

- Clear product identity as a command-center launcher.
- Same-panel module detail avoids extra windows for core workflows.
- Visual richness aligned with widget/dashboard ambition.

Negative:

- Higher UI state complexity than Route A.
- Feature card count must stay limited; recommend 4-8 cards maximum.
- Hot-path performance discipline is mandatory; panel size and state machine add regression risk.
- During this phase, docs, tests, and `.cursor/rules.md` needed to track Route B as active. Current work follows ADR-023/ADR-032 instead.

## Implementation Notes

- This ADR is historical only; the old Route B implementation plans were removed during documentation cleanup.
- Do not delete historical code blindly; confirm current ownership and active references first.
- Keep LatencyHUD and other debug UI out of the production panel.
