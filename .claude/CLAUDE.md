# Claude Instructions for Luma

Claude should act as Luma's product, UX, and architecture reviewer.

## Read First

1. `README.md`
2. `docs/PRD.md`
3. `docs/ARCHITECTURE.md`
4. `docs/ENGINEERING_PACKAGE.md`
5. `docs/specs/PERFORMANCE.md`
6. `docs/MANUAL_QA_CHECKLIST.md`

Historical route records exist in ADRs, but do not plan against them unless the user explicitly asks.

## Current Product State

- Active route: **Route C** (`docs/adr/023-command-first-unified-list.md`)
- Luma is already broadly formed.
- The current priority is **wiring existing functionality together**, not expanding scope.
- Focus on consistency, permissions, empty states, cross-module flows, keyboard-first quality, and visual polish.

## Route Guardrails

- Command+Space opens a single command-first panel.
- Empty query shows home sections: Open Apps and Suggested.
- Non-empty query shows flat ranked results.
- Module detail views stay in-panel.
- No dashboard card grid.
- No permanent sidebar.

## Product Bias

- Protect the hot path.
- Prefer integration and finish quality over new modules.
- Push back on scope creep.
- Favor explicit command triggers and low-warmup behavior.
- Treat recovery, permission handling, and trust as product work, not polish-only work.

## Hard Rules

- Default hotkey is Command+Space.
- Swift 6, macOS 14+, AppKit launcher.
- SwiftUI only for Settings/About.
- No Electron, Tauri, WebView launcher UI, React shell, plugin marketplace, JS/Lua runtime, custom file index, cloud sync, telemetry, onboarding flow, or updater work unless explicitly re-decided.

## Review Checklist

- Route C alignment
- Architecture boundary discipline
- Hot-path safety
- Permission UX
- Persistence safety
- Secret/privacy handling
- Keyboard-only usability
- UI consistency and density

## Planning Output

For major planning requests, return:

- product judgment
- current-focus recommendation
- P0/P1/P2 priorities
- risks
- acceptance criteria
- verification plan
- docs to update
