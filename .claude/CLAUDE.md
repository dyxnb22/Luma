# Claude Instructions for Luma

Claude should act as Luma's product, UX, and architecture reviewer.

## Read First

1. `README.md`
2. `docs/PRD.md`
3. `docs/ARCHITECTURE.md`
4. `docs/ENGINEERING_PACKAGE.md`
5. `docs/specs/MODULE_CONTRACT.md`
6. `docs/specs/PERFORMANCE.md`
7. `docs/MANUAL_QA_CHECKLIST.md`

Historical route records exist in ADRs, but do not plan against them unless the user explicitly asks.

## Current Product State

- Active route: **Route C** (`docs/adr/023-command-first-unified-list.md`)
- Luma is feature-complete and in final close-out / polish phase.
- The current priority is **finish quality**: consistency, permissions, empty states, cross-module flows, keyboard-first correctness, and visual polish.
- No new surface area. Push back on scope creep.

### Recently shipped (for review context)

- **Snippet trigger expansion**: typing a snippet's trigger word in global search and pressing Return expands and pastes inline — no detail navigation required. `LauncherRootController.activateReturn` intercepts before normal result dispatch.
- **Clipboard global search**: any 3+ character global query also searches clipboard history (≤ 3 results). `clip`/`cb` prefix still works as before.
- **Suggested section limits**: `ContextualHomeProvider` caps to 2 continue-flow + 1 create item (3 total). `HomeSuggestionMemory` gates eligibility.
- **Ranking exact-match boost**: `Ranker.score` adds +0.30 for items whose title exactly matches the query. Fuzzy weight reduced from 0.55 → 0.45 to accommodate.
- **Two-phase warmup**: `AppCoordinator` warms `BuiltInModules.fastModuleIDs` first → calls `setModulesReady(true)` → warms Notes/Projects in background.
- **Bug fixes**: `SnippetsAction.create` dead path restored; `SelectionSnapshotService` AX IPC moved off MainActor; `LauncherEnvironment.showStatus` promoted from optional var to non-optional let; `BrowserTabsIndex` empty-query dedup; `topTodoRow` stable memory key; `saveAsNote` correct `openAfterCapture: false`.

## Route Guardrails

- Command+Space opens a single command-first panel.
- Empty query shows home sections: Open Apps and Suggested (max 3 suggestions: 2 continue-flow + 1 create).
- Non-empty query shows flat ranked results; exact title match gets a +0.30 ranking boost.
- Typing a snippet trigger word and pressing Return expands the snippet inline without opening detail.
- Global search queries of 3+ characters surface up to 3 clipboard history matches alongside other results.
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
- Hot-path safety (no disk/network I/O per keystroke; no MainActor blocking; AX IPC off-main)
- Permission UX
- Persistence safety
- Secret/privacy handling
- Keyboard-only usability
- UI consistency and density
- `LauncherEnvironment.showStatus` must be called — it is a non-optional `let`, not an optional var; new code must inject it at init time
- Warmup correctness: filesystem-heavy modules (Notes, Projects, MenuItems, Media) belong in Phase 2; never add them to `BuiltInModules.fastModuleIDs`
- Snippet trigger expansion only fires in global search mode; must not intercept targeted module queries (`s`/`snip` prefix)

## Planning Output

For major planning requests, return:

- product judgment
- current-focus recommendation
- P0/P1/P2 priorities
- risks
- acceptance criteria
- verification plan
- docs to update
