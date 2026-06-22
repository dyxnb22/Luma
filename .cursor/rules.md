# Cursor Rules for Luma

Cursor should act as a precise implementation assistant for Luma. Do not reinterpret product direction while coding.

## Read First

Before non-trivial edits, read:

- `docs/strategy/PRODUCT_ROUTE_OPTIONS.md`
- `docs/ENGINEERING_PACKAGE.md`
- `docs/ARCHITECTURE.md`
- `docs/specs/PERFORMANCE.md`
- `docs/specs/MODULE_CONTRACT.md`

If the task is dashboard/widget UI work, also read:

- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
- `docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md`

If the task is pure launcher convergence work, also read:

- `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`
- `docs/strategy/CONVERGENCE_EXECUTION_PLAN.md`

## Product Route Guardrail

Luma has two documented product routes. Do not blend them accidentally.

- Route A: Launcher Convergence. Small pure launcher, no dashboard, no in-panel detail pages.
- Route B: Dashboard Widget Single Window. 860 x 540 liquid-glass panel, top search, left Open Apps sidebar, widget grid/results/detail in one panel.

Current accepted ADR: Route B via `docs/adr/007-dashboard-widget-single-window.md`.

Route B is the active UI implementation. Route A (pure launcher) is historical reference only. Do not revert to Route A patterns unless the user explicitly revives it with a new ADR.

## Cursor Composer Rules

- Use Composer in normal mode only.
- Do not enable Fast mode for this project.
- Execute one phase or one prompt at a time.
- Follow the named files, constants, dimensions, and acceptance checks exactly.
- Do not add adjacent features while implementing a prompt.
- If a prompt conflicts with the active route, stop and ask for a route decision.

## Hard Product Rules

- Default hotkey is Command+Space. Do not change it.
- If hotkey registration fails, show a visible menu bar warning. Do not auto-switch hotkeys.
- Do not convert the launcher to SwiftUI. AppKit owns the launcher panel.
- SwiftUI is allowed for Settings/About only.
- No Electron, Tauri, WebView primary UI, React, or frontend build tooling.
- No public plugin marketplace or JS/Lua runtime in v1.
- No custom file index. Use system facilities when file search is needed.
- Do not add cloud sync, telemetry, onboarding, or updater infrastructure.

## Architecture Boundaries

- `LumaCore`: pure models, protocols, ranking, actions. No AppKit.
- `LumaApp`: AppKit UI, coordinator, launcher window, settings.
- `LumaModules`: query/action modules behind `LumaModule`.
- `LumaServices`: system boundaries such as AX, Pasteboard, Translation, FSEvents, Keychain.
- `LumaInfrastructure`: logging, metrics, config, application support paths.

Do not call AX, Pasteboard, Keychain, NSWorkspace, or filesystem-heavy services directly from random UI code when an existing service boundary exists.

## Performance Rules

- Hotkey -> interactive panel p95 target: <= 50 ms.
- Keystroke -> first result paint p95 target: <= 30 ms.
- No disk or network I/O in per-keystroke hot paths.
- Module `handle` methods must be cancellation-aware and timeout-safe.
- Selection changes should not rebuild the entire result list.

## Verification

After Swift code changes:

```bash
swift build
```

After module, ranking, persistence, or action changes:

```bash
swift test
```

After bundle/runtime changes:

```bash
./scripts/build_app.sh
```

Do not claim completion unless the relevant command passes or you clearly report why it was not run.

