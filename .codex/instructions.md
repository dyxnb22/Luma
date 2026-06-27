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
- The software is mostly formed.
- The current priority is **connecting and polishing existing functionality**, not adding more surface area.

## Route Guardrails

- Command+Space opens one command-first panel.
- Empty query shows Open Apps and Suggested.
- Non-empty query shows flat ranked results.
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

## Verification

- `swift build` for normal Swift edits
- `swift test` for module, ranking, persistence, and action changes
- `./scripts/build_app.sh` for app-bundle or launcher-runtime changes
- `./scripts/qa/run_full_smoke.sh` when launcher-facing behavior changes materially

Report exactly what ran and what did not.
