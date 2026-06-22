# Roadmap

Active route: Route B (Dashboard Widget Single Window) per ADR-007.

Implementation reference:

- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
- `docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md`
- `docs/strategy/DASHBOARD_WIDGET_POLISH_PLAN.md`

Route A (`docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`, `CONVERGENCE_EXECUTION_PLAN.md`) is preserved as historical reference only; do not implement against it without superseding ADR-007.

## Current Strategic Roadmap

| Version | Focus | Do | Do Not Do |
| --- | --- | --- | --- |
| 0.1 Daily self-use | Dashboard Widget shell | Keep Route B single window, App Search, Window Focus, Clipboard, Commands, Calculator, Translate, same-panel details | Plugin API, Notes Graph, Wordbook, Secrets as first-class UX |
| 0.2 Stability | Persistence and settings | Namespaced Application Support persistence, interactive Settings, corrupt-data recovery, perf gates | XPC, runtime plugins, schema migration churn without need |
| 0.3 Command depth | Action depth | Secondary action chooser, better result grouping, stronger command ergonomics | Marketplace, JS/Lua runtime |
| 0.5 Architecture | Lifecycle and scale | ResidencyController, module lifecycle budgets, optional SQLite only if JSON files become insufficient | Premature XPC |
| 1.0 Maintainable app | Polish and distribution | Signed/notarized app, docs, repeatable release | Cloud sync, telemetry, broad public platform scope |

## Historical Phase Notes

These phase notes are preserved for context. Treat the current code and ADR-007 as authoritative when they differ.

## Phase 0: Skeleton

- Swift package and eventual Xcode project wiring.
- LSUIElement/accessory app setup.
- Carbon hotkey controller with Command+Space as the default.
- Menu bar item with Show Luma, Settings..., and Quit Luma.
- Pre-instantiated empty panel.
- Metrics hooks.
- `Features/` folder with per-module docs.

## Phase 1: Launcher and Card Shell

- `LauncherPanel`, `QueryField`, `ResultListView`, `ResultCellView`.
- Spotlight/Raycast-like AppKit visual language.
- Rounded card dashboard with edit buttons.
- Drag-to-reorder card position model.
- Keyboard navigation.
- Early fake static results, replaced by real module query dispatch.
- Snapshot diffing.
- Gate: fake keystroke -> paint p95 <= 30 ms.

## Phase 2: Core Data and Ranking

- Fuzzy matcher and ranker.
- Usage tracker.
- Local persistence. Current implementation uses JSON/UserDefaults/Keychain; SQLite remains optional, not active.
- Feature card layout persistence.
- Apps and Commands modules.
- First daily-dogfood build.

## Phase 3: Raycast-Like Workflows

- Translate module.
- Clipboard history module with secret filtering.
- Clipboard retention caps: 500 entries, 7 days, 100 KB per entry.
- Window Layouts module.
- Lazy Accessibility permission flow.

## Phase 4: Personal Knowledge and Secrets

- Secrets Vault with Keychain-backed encryption direction.
- Notes Graph module.
- Markdown vault tree.
- Typora open/edit integration.
- Backlink/tag graph index.

## Phase 5: Wordbook Migration

- Import `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- Preserve 9-stage review schedule.
- Review card with known/fuzzy/unknown actions.
- macOS speech for word/example.
- Daily goal/progress card.

## Phase 6: Polish and Release

- Settings.
- Multi-monitor/fullscreen Spaces verification.
- Developer ID signed and notarized DMG.
- Latency regression checklist.
- Local release script.

## Deferred

- Public plugin API.
- Cloud sync.
- Notes editor replacement.
- Complex animated graph rendering before the index is solid.
