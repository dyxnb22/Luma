# Codex Instructions for Luma

Codex should act as the coding agent for Luma: inspect first, make scoped edits, verify with local commands, and keep the product route coherent.

## Read First

For implementation work, read the relevant docs before editing:

- `docs/adr/023-command-first-unified-list.md` (active UI route)
- `docs/PRD.md`
- `docs/ENGINEERING_PACKAGE.md`
- `docs/ARCHITECTURE.md`
- `docs/specs/MODULE_CONTRACT.md`
- `docs/specs/PERFORMANCE.md`

Historical route docs (do not implement against unless explicitly revived):

- Route A: `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`, `docs/adr/006-launcher-convergence.md`
- Route B: `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`, `docs/adr/007-dashboard-widget-single-window.md`

## Route Discipline

Active route: **Route C — Command-First Unified List** (`docs/adr/023-command-first-unified-list.md`). Supersedes ADR-007.

Shape:

- Command+Space opens a single-column list (~700 × 480 pt).
- Empty query: home sections — Open Apps, Suggested, Recent.
- Non-empty query: flat ranked results from `QueryDispatcher`.
- Return runs primary action; Tab / ⌘K opens Action Panel; Esc unwinds home → close.
- Module detail views stay in the same panel; **no dashboard feature-card grid** and **no permanent sidebar**.

Do not blend Route B dashboard/widget patterns (card grid, 860 × 540 shell, sidebar-first navigation) into new work.

## Active Modules (high level)

Registered via `BuiltInModules.makeAll()`:

- Apps (root search), Clipboard (`clip`), Commands, Notes (`note`), Todo (`t`/`todo`), Events (`e`/`event`), Translate (`tr`/`translate`), Wordbook (`word`), Snippets (`s`/`snip`), Secrets (`secret`), Media (`m`/`media`), **Window Layouts** (`layout`/`win`/`wl`), **Projects** (`proj`/`p`/`project`).

Deferred (`makeDeferred()`): Calculator, Windows (window focus list).

Accessibility-dependent when active: Snippets, Window Layouts.

New command modules should stay **prefix-triggered**, **memory-indexed on query**, and **warmup-scanned** — not disk-scanned per keystroke.

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
