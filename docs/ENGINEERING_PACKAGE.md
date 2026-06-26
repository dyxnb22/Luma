# Engineering Package

## Current Entry Points

If you are new to the repo or preparing to make changes, read these documents first and treat them as the current source of truth:

1. [PRD](PRD.md) — product shape, active scope, built-in modules.
2. [Architecture](ARCHITECTURE.md) — runtime layers, launcher flow, module boundaries.
3. [ADR-023 Command-First Unified List](adr/023-command-first-unified-list.md) — active launcher route and home/search model.
4. [Opus Decisions](OPUS_DECISIONS.md) — enforced project decisions and product guardrails.
5. [Project Structure](PROJECT_STRUCTURE.md) — repo layout and ownership boundaries.
6. [Module Contract](specs/MODULE_CONTRACT.md) — module responsibilities and interfaces.
7. [Performance](specs/PERFORMANCE.md) — latency budgets and performance constraints.
8. [UX Behavior Rules](specs/UX_BEHAVIOR_RULES.md) — current launcher interaction rules.

Recommended reading order for most engineering work:

1. Product intent: [PRD](PRD.md)
2. Active UI route: [ADR-023](adr/023-command-first-unified-list.md)
3. System shape: [Architecture](ARCHITECTURE.md)
4. Constraints: [Module Contract](specs/MODULE_CONTRACT.md), [Performance](specs/PERFORMANCE.md), [UX Behavior Rules](specs/UX_BEHAVIOR_RULES.md)
5. Repo navigation: [Project Structure](PROJECT_STRUCTURE.md)

## Historical Documents

Some documents are intentionally retained for implementation archaeology. They are useful for understanding how the project evolved, but they are not current product or UX guidance.

Do not use these as the primary basis for new changes unless you are explicitly researching old behavior:

- [ADR-007 Dashboard Widget Single Window](adr/007-dashboard-widget-single-window.md)
- `docs/strategy/DASHBOARD_WIDGET_*`
- `docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md`
- `docs/strategy/NOTES_CURSOR_PROMPTS.md`

When a historical document conflicts with an active document, follow:

1. [ADR-023](adr/023-command-first-unified-list.md)
2. [PRD](PRD.md)
3. [Architecture](ARCHITECTURE.md)
4. The current code

## Current Direction Summary

- Active launcher route is Route C: command-first unified list.
- Empty query shows home sections, not a dashboard card grid.
- Module details open in the same panel.
- User-facing media functionality is named Records; the technical module identifier remains `luma.media`.

## Implementation Defaults

- Swift 6 strict concurrency.
- macOS 14+.
- AppKit primary launcher UI.
- SwiftUI only for Settings/About.
- In-process modules only for v1.
- JSON persistence first; migrate to SQLite only after explicit data-size thresholds.
- `os_signpost`-style metrics from Phase 0.

## ADR Index

- [ADR-001 Swift + AppKit for Primary UI](adr/001-swift-appkit-primary-ui.md)
- [ADR-002 Pre-Instantiated Launcher Panel](adr/002-preinstantiated-panel.md)
- [ADR-003 Actor-Based Module System](adr/003-actor-module-system.md)
- [ADR-004 In-Process Modules for v1](adr/004-in-process-modules-v1.md)
- [ADR-005 Carbon Global Hotkey](adr/005-carbon-global-hotkey.md)
- [ADR-006 Launcher Convergence](adr/006-launcher-convergence.md)
- [ADR-007 Dashboard Widget Single Window](adr/007-dashboard-widget-single-window.md) — superseded by ADR-023
- [ADR-023 Command-First Unified List (Route C)](adr/023-command-first-unified-list.md) — **active UI route**

## Module Contract

See [Module Contract](specs/MODULE_CONTRACT.md).

## Performance Contract

See [Performance](specs/PERFORMANCE.md).

## UX Rules

See [UX Behavior Rules](specs/UX_BEHAVIOR_RULES.md).

## Milestones

See [Roadmap](ROADMAP.md).

## Opus Decisions

See [Opus Decisions](OPUS_DECISIONS.md), [Product Route Options](strategy/PRODUCT_ROUTE_OPTIONS.md), and [Launcher Convergence Strategy](strategy/LAUNCHER_CONVERGENCE_STRATEGY.md).
