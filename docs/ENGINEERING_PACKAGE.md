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
9. **[Launcher Home Constraints](specs/LAUNCHER_HOME_CONSTRAINTS.md)** — frozen empty-query home; **required before any home UI change**.
10. **[Launcher Panel Constraints](specs/LAUNCHER_PANEL_CONSTRAINTS.md)** — panel geometry, positioning, in-panel layout; **required before panel/chrome changes**.
11. **[Launcher Navigation Audit](qa/LAUNCHER_NAVIGATION_AUDIT.md)** — temporary open-issue register (navigation, shortcuts, session); align specs when closing items.
12. [Manual QA Checklist](MANUAL_QA_CHECKLIST.md) — current product-review and regression checks.
13. [Recorded QA Brief](RECORDED_QA_BRIEF.md) — recorded walkthrough scope and findings format.
14. [Integration P0](INTEGRATION_P0.md) — current short list for wiring together existing functionality.

Recommended reading order for most engineering work:

1. Product intent: [PRD](PRD.md)
2. Active UI route: [ADR-023](adr/023-command-first-unified-list.md)
3. Workbench direction: [WORKBENCH_STRATEGY.md](WORKBENCH_STRATEGY.md)
4. System shape: [Architecture](ARCHITECTURE.md)
4. Constraints: [Launcher Home Constraints](specs/LAUNCHER_HOME_CONSTRAINTS.md), [Launcher Panel Constraints](specs/LAUNCHER_PANEL_CONSTRAINTS.md), [Module Contract](specs/MODULE_CONTRACT.md), [Performance](specs/PERFORMANCE.md), [UX Behavior Rules](specs/UX_BEHAVIOR_RULES.md)
5. Repo navigation: [Project Structure](PROJECT_STRUCTURE.md)

## Priority Order

When documents conflict, follow:

1. [Launcher Home Constraints](specs/LAUNCHER_HOME_CONSTRAINTS.md) — for empty-query home
2. [Launcher Panel Constraints](specs/LAUNCHER_PANEL_CONSTRAINTS.md) — for panel geometry, positioning, and in-panel layout
3. [ADR-023](adr/023-command-first-unified-list.md)
4. [PRD](PRD.md)
5. [Architecture](ARCHITECTURE.md)
6. The current code

## Current Direction Summary

- Active launcher route is Route C: command-first unified list.
- Empty query shows **Open Apps only** on home (frozen).
- Module details open in the same panel.
- User-facing media functionality is named Records; the technical module identifier remains `luma.media`.
- Auto Workflow is a default-off, on-demand wrapper around the external `cc-loop` CLI; see ADR-031.
- The product is already broadly built; current priority is integration and polish over new scope.

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
- [ADR-031 Auto Workflow cc-loop Integration](adr/031-autoworkflow-integration.md)

## How to Add a New Module

1. **Create the module actor** in `Sources/LumaModules/<Name>/`. Implement `LumaModule`: `manifest`, `warmup`, `handle`, `perform`, `teardown`.
2. **Create `<Name>ModuleBundle.swift`** in the same folder with `manifest`, `warmupTier`, `commands`, optional `detailMetadata` / `presentation` / `defaultOffNote`, and `makeModule()`.
3. **Register the bundle** — add `NameModuleBundle.self` to `ModuleRegistry.allBundles` in `Sources/LumaModules/ModuleRegistry.swift`.
4. **Add detail view** (if needed) in `Sources/LumaApp/Launcher/<Name>DetailView.swift` and register the factory in `ModuleDetailRegistry.makeDefault()`. Follow [Launcher Panel Constraints](specs/LAUNCHER_PANEL_CONSTRAINTS.md) — prefer `BaseDetailContainer`, no `wantsLayer` on full-width roots, pin custom layouts to container width. Module shortcuts must work when detail subviews hold focus (see [Navigation Audit](qa/LAUNCHER_NAVIGATION_AUDIT.md) MOD-KB).
5. **Write tests** in `Tests/LumaModulesTests/` and run `swift test`.

Warmup tiers:

- `hotPath` — participates in global search fan-out; warms at startup when pinned (default for in-memory modules).
- `onDemand` — excluded from global search; warms on first targeted query or detail open (examples: Notes, Projects, MenuItems, Auto Workflow).

Users can pin modules in Settings → Modules for always-hot startup behavior and workbench gating (`enabled ∩ pinned`). Pinning does **not** add home rows.

Home and cross-module rules:

- **Do not** restore setup/recent/continue/create home sections or wire removed home providers into `LauncherHomeAggregator` without a new ADR — see [Launcher Home Constraints](specs/LAUNCHER_HOME_CONSTRAINTS.md).
- Add workbench behavior through `WorkbenchContextBuilder`, capture services, and command routers for **command and detail** surfaces only.
- Keep cross-module draft construction behind narrow helpers/protocols such as `ProjectContextSuggestions` or `QuicklinkDraftSource`.
- Do not add new App-layer switches for commands, feature cards, or detail metadata; read those from `ModuleRegistry` / `ModuleDetailRegistry`.

## Workbench Core

Location: `Sources/LumaCore/Workbench/`

| Type | Role |
| --- | --- |
| `WorkbenchContext` | Assembled user snapshot for capture/command/detail |
| `WorkbenchCaptureService` | Protocol; implementation in `LumaApp/Composition/` |
| `WorkbenchActivityStore` | Local activity trail for Continue/Resume |
| `WorkbenchCommandRouter` | `cap clip/sel …` routing before global search |

Wiring rules:

- Build context in `WorkbenchContextBuilder` on launcher activation; pass into workbench capture and command surfaces.
- Route captures through `DefaultWorkbenchCaptureService`; do not scatter draft builders in UI controllers.
- Workbench commands check `enabledModuleIDs` before capture; never fan out to global search.
- Modules may expose capture targets via narrow draft builders (`SnippetDraft+Clipboard`, `ProjectContextSuggestions`).

See [WORKBENCH_STRATEGY.md](WORKBENCH_STRATEGY.md).

Key constraints from [Module Contract](specs/MODULE_CONTRACT.md):
- `handle` must be memory-only (no disk/network I/O).
- `handle` must respect query cancellation.
- State must stay private to the module actor.
- Do not reach into other modules or touch AppKit views.

## Module Contract

See [Module Contract](specs/MODULE_CONTRACT.md).

## Performance Contract

See [Performance](specs/PERFORMANCE.md).

## UX Rules

See [UX Behavior Rules](specs/UX_BEHAVIOR_RULES.md).

## Milestones

See [Roadmap](ROADMAP.md).
