# Launcher Convergence Execution Plan

Status: active planning backlog  
Date: 2026-06-22  
Companion: `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`

## P0: This Week

These tasks make Luma a focused launcher instead of a dashboard.

| Priority | Task | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| P0.1 | Remove dashboard cards from the launcher panel. | `LauncherRootView.swift`, `FeatureCatalog.swift` | Empty query shows only usage-based recents/frequents. |
| P0.2 | De-emphasize or disable non-core modules in the default registry. | `BuiltInModules.swift`, `FeatureCatalog.swift`, settings defaults | Notes, Wordbook, Secrets, Window Layout, Dashboard Cards no longer shape first-run UX. |
| P0.3 | Keep Command+Space behavior stable. | `HotkeyConfig.swift`, `HotkeyController.swift` | Command+Space remains default and menu warns if blocked. |
| P0.4 | Make empty query recents purely usage-backed. | `UsageResultCache`, `PersistentUsageTracker`, `LauncherRootView.swift` | No NSDocumentController fallback in the primary empty state. |
| P0.5 | Hide panel immediately on action dispatch. | `LauncherRootView.swift`, `LauncherWindowController.swift` | Return hides panel before slow action completion. |
| P0.6 | Make LatencyHUD debug-only or hideable. | `LatencyHUD.swift`, `LauncherRootView.swift`, config | Release UI has no debug HUD unless explicitly enabled. |

## P1: Next Two Weeks

| Priority | Task | Acceptance |
| --- | --- | --- |
| P1.1 | Introduce `PersistenceStore` protocol with JSON implementation. | Usage and clipboard can request namespaced files through one boundary. |
| P1.2 | Move configuration toward a single namespaced JSON model. | Module settings no longer spread across unrelated UserDefaults keys. |
| P1.3 | Add real action chooser overlay on Tab. | Tab opens secondary action UI; Esc returns to results. |
| P1.4 | Persist app index cache. | Warmup can render cached apps before background rescan. |
| P1.5 | Add corrupt persistence recovery. | Bad JSON is renamed `.corrupted`, app continues with empty state. |
| P1.6 | Keep 1000-keystroke dispatcher replay as a performance gate. | Warm p95 remains <= 30 ms. |

## P2: One To Two Months

| Priority | Task | Acceptance |
| --- | --- | --- |
| P2.1 | Add `SpotlightService` around system file search. | File search uses system index; Luma does not build its own index. |
| P2.2 | Add `ScriptedCommandsModule`. | YAML commands in Application Support can run shell/AppleScript/Shortcut actions. |
| P2.3 | Add `EventBus`. | Usage/config/memory events have a single lightweight route. |
| P2.4 | Add `ResidencyController`. | Modules have clear warm/hot/cold lifecycle tiers. |
| P2.5 | Tune frecency with real usage. | Empty query results consistently reflect actual behavior. |
| P2.6 | Stabilize local release. | `.app` bundle, signing path, and launchd scripts are documented and repeatable. |

## P3: Long Term

| Priority | Task | Trigger |
| --- | --- | --- |
| P3.1 | SQLite implementation for `PersistenceStore`. | JSON thresholds exceeded. |
| P3.2 | XPC isolation. | User scripts/plugins require unsafe execution boundaries. |
| P3.3 | Public distribution polish. | The app is useful to someone beyond the primary user. |

## Explicit Non-Goals For This Plan

- No plugin marketplace.
- No JS/Lua runtime.
- No dashboard-first UX.
- No in-panel details.
- No Notes Graph.
- No Wordbook in the launcher path.
- No first-class password manager scope.
- No window tiling engine.
- No custom file index.

## Verification Matrix

| Area | Test |
| --- | --- |
| Build | `swift build` |
| Unit/integration | `swift test` |
| Release bundle | `./scripts/build_app.sh` |
| Hot path | 1000-keystroke replay p95 <= 30 ms |
| Manual QA | `docs/MANUAL_QA_CHECKLIST.md` |
| Visual QA | Compare launcher against fixed 720 x 440 pure-result layout |

