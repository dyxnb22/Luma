# Engineering Package

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
