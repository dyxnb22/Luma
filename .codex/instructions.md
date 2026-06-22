# Codex Instructions for Luma

Codex should act as the coding agent for Luma: inspect first, make scoped edits, verify with local commands, and keep the product route coherent.

## Read First

For implementation work, read the relevant docs before editing:

- `docs/strategy/PRODUCT_ROUTE_OPTIONS.md`
- `docs/ENGINEERING_PACKAGE.md`
- `docs/ARCHITECTURE.md`
- `docs/specs/MODULE_CONTRACT.md`
- `docs/specs/PERFORMANCE.md`
- `docs/adr/007-dashboard-widget-single-window.md`

For route-specific work:

- Route A: `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`
- Route B: `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`

## Route Discipline

There are two documented routes:

- Route A: Launcher Convergence. Pure launcher, usage-backed empty state, result-focused panel.
- Route B: Dashboard Widget Single Window. Liquid-glass dashboard panel with sidebar, widget grid, results overlay, and same-panel details.

Current accepted ADR: Route B via `docs/adr/007-dashboard-widget-single-window.md`.

Do not blend both routes in code. Route B is the active implementation route; Route A is historical reference only unless the user explicitly revives it with a new superseding ADR.

## Non-Negotiables

- Default hotkey remains Command+Space.
- Do not replace AppKit launcher UI with SwiftUI.
- SwiftUI is allowed only for Settings/About.
- No Electron, Tauri, WebView, React, or frontend build stack.
- No public plugin API, marketplace, JS runtime, or Lua runtime in v1.
- No custom file index.
- No cloud sync, telemetry, onboarding flow, or updater infrastructure.
- Do not revert user changes unless explicitly asked.

## Engineering Boundaries

- Prefer existing patterns and target boundaries.
- Keep `LumaCore` free of AppKit and system UI dependencies.
- Keep system integrations in `LumaServices`.
- Keep query/action behavior inside modules or core abstractions.
- Keep UI code focused on rendering, navigation, and user interaction.
- Avoid broad refactors unless needed for the requested change.

## Hot Path Rules

- No disk or network I/O per keystroke.
- Query tasks must cancel cleanly on new input.
- Module timeouts must be respected.
- Result row dimensions should be stable.
- Keyboard selection updates should be local and cheap.
- Panel should hide before slow action completion.

## Verification

Use the narrowest sufficient verification:

- `swift build` for normal Swift edits.
- `swift test` for core, module, persistence, ranking, and action changes.
- `./scripts/build_app.sh` for bundle, launchd, LSUIElement, or release-shell changes.

Report commands run and failures honestly.

## Review Posture

When asked for review, lead with bugs and risks, ordered by severity, with file/line references. Prioritize route drift, hot-path latency, persistence corruption, permissions, security leaks, and missing tests.
