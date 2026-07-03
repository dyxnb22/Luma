# Codex Instructions for Luma

Codex should act as Luma's implementation agent: inspect first, edit narrowly, verify honestly, and keep the product coherent.

## Read First

- `README.md`
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/ENGINEERING_PACKAGE.md`
- `docs/specs/MODULE_CONTRACT.md`
- `docs/specs/PERFORMANCE.md`
- `docs/MANUAL_QA_CHECKLIST.md`

## Current Product State

- Active route: **Route C** (`docs/adr/023-command-first-unified-list.md`)
- Luma is feature-complete and in final close-out / polish phase.
- The current priority is **finish quality**: consistency, permission UX, empty states, and cross-module correctness — not adding new surface area.

## Route Guardrails

- Command+Space opens one command-first panel.
- Empty query shows **Open Apps only** (frozen 2026-07-03; see `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`).
- Non-empty query shows flat ranked results; exact title match gets +0.30 ranking boost.
- Typing a snippet trigger word in global search + Return expands and pastes inline (no detail view).
- Global queries of 3+ chars also search clipboard history (≤ 3 results).
- Return runs primary action.
- Tab / `⌘K` opens secondary actions.
- Module detail stays in the same panel.
- No dashboard card grid.
- No permanent sidebar.

## Implementation Bias

- Prefer integration fixes over new modules.
- Improve permission handling, empty states, recovery, cross-module flows, and consistency.
- Keep command modules prefix-triggered, warmup-indexed, and memory-only on the query path.
- Avoid broad refactors unless they unblock the current request.

## Non-Negotiables

- Default hotkey remains Command+Space.
- AppKit owns the launcher.
- SwiftUI only for Settings/About.
- No Electron, Tauri, WebView, React shell, plugin marketplace, JS/Lua runtime, custom file index, cloud sync, telemetry, onboarding flow, or updater work unless explicitly requested.
- Do not revert user changes unless explicitly asked.

## Engineering Guardrails

- Keep `LumaCore` UI-free.
- Keep system boundaries in `LumaServices`.
- Keep launcher UI in `LumaApp`.
- No disk or network I/O per keystroke.
- Respect query cancellation and module timeouts.
- Keep row height, selection, and panel dismissal behavior stable.
- `LauncherEnvironment.showStatus` is a non-optional `let (String) -> Void`; inject at init, never assign post-construction.
- Filesystem-heavy modules (Notes, Projects, MenuItems, Media) and external-CLI modules (Auto Workflow) must **not** be added to `BuiltInModules.fastModuleIDs`; they belong outside the global hot path.
- AX IPC (accessibility API calls) must run off the MainActor; only PID/frontmostApplication capture is allowed on MainActor.
- `WorkbenchContextBuilder` loads activity and link stores in parallel; do not add unnecessary sequential `await` calls there.
- New `LauncherEnvironment` callbacks must be injected as `let` parameters in `init`, not set as optional `var` after construction.

## Verification

- `swift build` for normal Swift edits
- `swift test` for module, ranking, persistence, and action changes
- `./scripts/build_app.sh` for app-bundle or launcher-runtime changes
- `./scripts/qa/run_full_smoke.sh` when launcher-facing behavior changes materially

Report exactly what ran and what did not.
