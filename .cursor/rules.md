# Cursor Rules for Luma

Cursor should be a precise implementation assistant for Luma. Keep edits scoped and aligned with the current product.

## Read First

- `README.md`
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/ENGINEERING_PACKAGE.md`
- `docs/specs/MODULE_CONTRACT.md`
- `docs/specs/PERFORMANCE.md`
- `docs/MANUAL_QA_CHECKLIST.md`

## Current Focus

- Active route: **Route C** (`docs/adr/023-command-first-unified-list.md`)
- Luma is feature-complete and in final close-out / polish phase.
- Current work should prioritize **finish quality**: fixing rough edges, improving trust, closing empty states, and consistency — not adding new surface area.

## Route Guardrail

- Single command-first panel
- Empty query: Open Apps + Suggested (max 2 continue-flow + 1 create)
- Non-empty query: flat results; exact title match gets +0.30 ranking boost
- Snippet trigger word in global search + Return → inline expansion, no detail
- Global queries ≥ 3 chars surface up to 3 clipboard entries alongside other results
- Same-panel detail views
- No dashboard card grid
- No permanent sidebar

## Composer Rules

- Use normal mode only.
- Do not enable Fast mode.
- Do one scoped task at a time.
- Do not add adjacent features unless explicitly requested.
- If a request conflicts with Route C, stop and ask.

## Product Rules

- Default hotkey is Command+Space.
- AppKit owns the launcher.
- SwiftUI only for Settings/About.
- No Electron, Tauri, WebView launcher UI, React shell, plugin marketplace, JS/Lua runtime, custom file index, cloud sync, telemetry, onboarding, or updater work.

## Implementation Rules

- Prefer integration fixes over new modules.
- Improve permissions, recovery, empty states, cross-module flows, and keyboard-first behavior.
- Keep heavy work out of the query hot path.
- Use existing service boundaries.
- Do not filesystem-scan on each keystroke.
- `LauncherEnvironment.showStatus` is `let (String) -> Void` — inject at init, never post-assign.
- AX IPC calls must not run on the MainActor; only PID capture (`frontmostApplication`) is allowed on main.
- `BuiltInModules.fastModuleIDs` is Phase 1 warmup; Notes/Projects/MenuItems/Media/Auto Workflow stay out of the hot path.
- `ContextualHomeProvider.rankedSectionItems` uses `async let` for all 9 fetches — keep them concurrent.
- New `LauncherEnvironment` callbacks must be `let` parameters in `init`, not optional `var`.

## Verification

- `swift build`
- `swift test` for module, ranking, persistence, and action changes
- `./scripts/build_app.sh` for runtime or bundle changes
- `./scripts/qa/run_full_smoke.sh` for meaningful launcher UX changes

Do not claim completion without honest verification status.
