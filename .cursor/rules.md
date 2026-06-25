# Cursor Rules for Luma

Cursor should act as a precise implementation assistant for Luma. Do not reinterpret product direction while coding.

## Read First

Before non-trivial edits, read:

- `docs/adr/023-command-first-unified-list.md` (active UI route)
- `docs/PRD.md`
- `docs/ENGINEERING_PACKAGE.md`
- `docs/ARCHITECTURE.md`
- `docs/specs/PERFORMANCE.md`
- `docs/specs/MODULE_CONTRACT.md`

Historical only — do not implement unless the user revives with a new ADR:

- Route A: `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`, `docs/adr/006-launcher-convergence.md`
- Route B: `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`, `docs/adr/007-dashboard-widget-single-window.md`

## Product Route Guardrail

Active route: **Route C — Command-First Unified List** (`docs/adr/023-command-first-unified-list.md`).

- Single-column list; empty query shows Open Apps / Suggested / Recent sections.
- Search yields flat `QueryDispatcher` results; Return runs primary action.
- Tab / ⌘K opens Action Panel; module details stay in the same panel.
- **No dashboard feature-card grid** and **no permanent sidebar** on the home screen.

Do not reintroduce Route B patterns (860 × 540 widget grid, sidebar-first layout, card-jump ⌘N semantics) unless the user explicitly supersedes ADR-023.

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

## Module Conventions

Active built-ins live in `BuiltInModules.makeAll()`. Deferred: Windows.

Recent command-first modules to mirror when adding features:

- **Window Layouts** (`layout` / `win` / `wl`) — prefix-only trigger, in-memory command catalog, Accessibility required.
- **Projects** (`proj` / `p` / `project`) — `~/Library/Application Support/Luma/projects.json`, warmup index + shallow root scan, no per-keystroke filesystem scan.

`FeatureCatalog.dashboardCoreCards()` is legacy metadata for detail headers; it is not the home-screen entry model under Route C.

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
