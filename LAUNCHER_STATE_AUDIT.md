# Launcher Session State Audit (Phase 12.1)

**Date:** 2026-07-07  
**Baseline:** `803b0672` + uncommitted Phase 11–12 work  
**Scope:** Facts only — no production wiring changes in this slice.

**Sources:** `LauncherSessionState.swift`, `LauncherRootController.swift`, `LauncherDetailPresenter.swift`, `LauncherWindowController.swift`, `LauncherPanelVisibilitySession.swift`, `LauncherSessionStateTransitionTests.swift`, `LAUNCHER_STATE_OWNER_MAP.md`.

---

## 1. Reducer overview

`LauncherSessionState` (`Sources/LumaCore/Launcher/Session/LauncherSessionState.swift`) is a **shadow reducer** with three axes:

| Axis | Type | Production SoT today |
|------|------|---------------------|
| `panel` | `LauncherPanelPhase` | **`LauncherPanelVisibilitySession`** + `LauncherWindowController` |
| `content` | `LauncherContentPhase` | **`LauncherContentCoordinator.mode`** |
| `detailMode` | `LauncherDetailModePhase` | **`LumaSearchBar` detail mode** + **`LauncherContentCoordinator`** detail presentation |

Effects emitted: `cancelAllTasks`, `clearDetailModeState` → applied by `LauncherSessionEffectApplier` in `LauncherRootController`.

---

## 2. Event inventory

| Event | Reducer effect on state | Side effects | Production wired? | Production call site |
|-------|-------------------------|--------------|-------------------|----------------------|
| `panelShowBegan` | `hidden/hiding → showing` | none | **No** | — |
| `panelShowCompleted` | `showing → visible`; `panelGeneration += 1` | none | **Yes** | `LauncherRootController.markPanelActiveForSnapshotApply()` ← `activatePanelForQueryApply()` (show deferred work) |
| `panelHideBegan` | `visible/showing → hiding` | `cancelAllTasks` | **Yes** | `markPanelInactiveForSnapshotApply()` ← `cancelActiveQueryAndSnapshotApply()` (panel hide) |
| `panelHideCompleted` | `→ hidden`; `panelGeneration += 1` | none | **No** | — |
| `queryBecameEmpty` | `results → home` | none | **No** | Empty query handled in `handleTextChange` → `transitionToEmptyQueryHome()` without session event |
| `queryBecameNonempty` | `→ results` (blocked in detail) | none | **No** | `handleTextChange` → `transitionToResultsMode()` without session event |
| `detailOpenRequested` | `detailMode → active`; `content → detail` | none | **No** | `LauncherDetailPresenter` opens detail directly |
| `detailOpened` | `content → detail`; `detailMode → active(suspended)` | none | **Yes** | `LauncherDetailPresenter` after presentation |
| `detailExitRequested` | `detailMode → exiting(outcome)` | none | **No** | `exitDetailFromChrome` uses `LauncherDetailExitPlanner` → `applyDetailExitOutcome` without session event |
| `detailClosed` | `detailMode → inactive`; `content → home` if detail | `clearDetailModeState` | **Yes** | `closeDetail` tearDown via `detailLifecycle.onTearDown` |
| `userTypedInDetail` | `detailMode → inactive`; `content → results` | `clearDetailModeState` | **No** | `dismissDetailForNewQuery()` calls `cancelDetailMode` + `closeDetail` without session event |

**Summary:** 4 / 11 events wired in production. 7 events exist only in `LauncherSessionStateTransitionTests` (plus illegal-transition I1–I7 tests).

---

## 3. Duplication with real owners

| Shadow field | Mirrors | Divergence risk |
|--------------|---------|-----------------|
| `sessionState.panel` | `visibilitySession.isVisible` + hide fade | **High.** `panelHideBegan` fires at hide start; `visibilitySession.isVisible` flips false immediately in `beginHide()`. `panelHideCompleted` never fires — shadow can stay `.hiding` forever after hide finishes. |
| `sessionState.content` | `LauncherContentCoordinator.mode` | **High.** Query transitions never update reducer; coordinator is authoritative. |
| `sessionState.detailMode` | `LumaSearchBar` suspend + coordinator `showingDetail` | **Medium.** `detailOpened` / `detailClosed` partially wired; typing exit and planner exit bypass `detailExitRequested` / `userTypedInDetail`. |
| `panelGeneration` | `visibilitySession.generation` | **High.** Separate counters; not synchronized. |

---

## 4. Three visibility signals

| Signal | Location | When true / meaning | Used for |
|--------|----------|---------------------|----------|
| **`visibilitySession.isVisible`** | `LauncherPanelVisibilitySession` | Set `true` in `beginShow()`; `false` in `beginHide()` **before** fade completes | Hotkey guards, `isPanelVisible`, background home refresh gate, external-activation hide |
| **`sessionState.panel`** | `LauncherSessionState` | Reducer phase: `hidden/showing/visible/hiding` | Snapshot apply gating side effects only; **not** read for show/hide decisions |
| **`panel.isVisible`** (AppKit) | `LauncherPanel` (`NSPanel`) | AppKit window on-screen flag | `LauncherPanel.performKeyEquivalent` hide-hotkey guard only (`LauncherPanel.swift:146`) |

**During hide fade:** `visibilitySession.isVisible == false` while `panel.isVisible` may still be true until `orderOut`. `sessionState.panel` is `.hiding` after `panelHideBegan` but never reaches `.hidden` in production (no `panelHideCompleted`).

**After Phase 11.5:** `refreshHomeForBackgroundDataUpdate` uses `visibilitySession.isVisible`, not AppKit `panel.isVisible`.

---

## 5. Effect wiring

| Effect | Applier action | Production trigger |
|--------|----------------|-------------------|
| `cancelAllTasks` | `cancelLauncherAsyncWork()` | `panelHideBegan` only |
| `clearDetailModeState` | `searchBar.cancelDetailMode()` | `detailClosed` only |

**Gap:** `userTypedInDetail` would also emit `clearDetailModeState` in tests, but production typing path does not fire the event (calls cancel/close directly).

---

## 6. Recommendation (Phase 12.1 — no code change)

| Option | Verdict |
|--------|---------|
| **Full production wiring** | **Not recommended now.** Would duplicate `LauncherContentCoordinator` / `visibilitySession` and require reconciling `panelHideCompleted`, query events, and detail exit planner — high conflict with Phase 11 boundaries, low P0 user value. |
| **Delete shadow reducer** | **Defer to P2.** Needs migration plan for the 4 wired events and effect applier; tests document intended illegal transitions useful for a future unified owner. |
| **Keep test-only + narrow production use** | **Recommended for Phase 12.** Treat `LauncherSessionState` as **specification / test helper** for illegal-transition rules. Production continues using `visibilitySession` + `contentCoordinator` as SoT. Document the 4 wired events as **legacy side-effect hooks** (cancel on hide, clear detail on close, snapshot apply activation). |

**Phase 12.3+ action:** Do **not** wire remaining 7 events without a P1.2 design pass. Add comments at `applySessionEvent` call sites referencing this audit.

**P2 decision (Phase 15, 2026-07-07):** `LauncherSessionState` remains **test-only specification + 4 legacy production effect hooks** through P2. No new `applySessionEvent` call sites in P2 slices. Delete-vs-promote decision deferred to P2.5+ / P3 (`P2_DECISION_MATRIX.md` §1).

---

## 7. Test coverage

| Area | Tests | Gap |
|------|-------|-----|
| Illegal transitions | `LauncherSessionStateTransitionTests` I1–I7 | Good for reducer spec |
| Production wiring parity | `ModuleDisableWiringTests` (string guards) | No behavioral test that shadow `panel` matches `visibilitySession` after hide |
| Effect applier | None dedicated | `cancelAllTasks` on hide covered indirectly by hide tests |

---

*Phase 12.1 — documentation only.*
