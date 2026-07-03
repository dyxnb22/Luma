# Launcher Navigation & Interaction Audit (Temporary)

**Status:** Temporary working document — 2026-07-03 (implementation pass)  
**Purpose:** Track navigation, input, shortcuts, commands, and layout gaps.  
**Authority:** Canonical behavior in `docs/specs/UX_BEHAVIOR_RULES.md` and `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`.

---

## Summary

| Priority | Fixed | Open |
| --- | ---: | ---: |
| P0 | 3 | 0 |
| P1 | 12 | 0 |
| P2 | 12 | 1 |

**Remaining open:** L4 (spec drift — doc-only, already aligned).

---

## Issue register

### P0 — fixed

| ID | Fix |
| --- | --- |
| K6 | `LauncherKeyRouter` returns `.handled` for ↑↓ / ⌘1–9 in `.detail` mode; read-only search field blocks list keys |
| K1 | `LauncherPanel.onCloseDetail` — ⌘W calls `exitDetailFromChrome` |
| L1 | `LauncherOverlayHostView` + `LauncherListView.passesHitTests` / `hitTest` during cross-fade |

### P1 — fixed

| ID | Fix |
| --- | --- |
| MOD-KB | `LauncherPanel.onDetailKeyDown` → `dispatchDetailKeyDown` forwards to `ModuleDetailView.handleKeyDown` |
| K4 | `handleEscape` tries `dispatchDetailKeyDown(Esc)` before `exitDetailFromChrome` (Notes mind-map → outline) |
| K3 | `LauncherListView` maps ⌘↩ → `.commandReturn` |
| K7 | `handleKeyCommand` routes ↑↓ to `actionPanel` when visible; list `onInterceptKeyDown` |
| K2 | N/A in detail — `LauncherHintBar` uses `hint.detail.back` only (no ⌘↩ line) |
| CD1 | `closeDetailIfShowing` → `exitDetailFromChrome` |
| S1/S2 | `searchBar.persistedQuery` saved in `saveCurrentSession` / `persistResumeState` |
| S3 | `applyRestore(.openModule)` — removed racy sync translate restore; `presentModuleDetail` handles it |
| L2 | `PermissionBannerController` — layer on pinned `chromeView` child |
| L3 | Clipboard, Secrets, Todo toolbars use `constrainDetailToolbarTrailingActions` |
| NAV-01 | `restoreLastSessionIfNeeded` re-focuses when `showingDetail`; `focusSearchFieldAfterShow` defers search focus |
| NAV-02 | Detail back/close → `exitDetailFromChrome` (prior fix) |

### P2 — fixed

| ID | Fix |
| --- | --- |
| I1 | `beginDetailMode` resigns search first responder; read-only `keyDown` blocks list keys |
| I2 | `closeDetail` → `clearStuckDetailModeState` |
| I3 | `focusSearchFieldAfterShow` on panel show |
| K5 | Search hints popover expanded |
| K8 | `launcher.detail.placeholder` L10n key |
| L5 | `onPanelSpacingChanged` → `stabilizePanel` |
| L6 | `stabilizePanel` in animation completion handlers |
| S5 | `tearDownDetailIfNeeded` fires `onHomeSessionSaved` |
| TR-01 | `openTranslateDetail` routes through `openModuleDetail` (warmup + reserve) |
| DEAD-01 | `onBackFromDetail` → `closeDetailIfShowing` → `exitDetailFromChrome` |
| L7 | `LauncherActionPanel.configureLayout` + `reposition(relativeTo:)`; `openActionPanel` anchors to selected row |
| C1 | `WorkbenchCommandRouter.commandHint` + `handleTextChange` applies workbench hint when `workbenchRoute != .none` |

### Still open

| ID | Notes |
| --- | --- |
| L4 | Spec aligned (prior doc pass) — no code change |

---

## Changelog

| Date | Change |
| --- | --- |
| 2026-07-03 | Initial audit |
| 2026-07-03 | NAV-02 — detail chrome exit |
| 2026-07-03 | Implementation pass — P0/P1/P2 items above |
| 2026-07-03 | ADR-032 — split home; module detail in right column; Open Apps stay left |
