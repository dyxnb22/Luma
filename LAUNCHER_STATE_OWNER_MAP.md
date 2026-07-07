# Launcher State Owner Map (Phase 11.1)

**Date:** 2026-07-07  
**Baseline:** `803b0672` — *Document P0 smoke gate and phase 9 exit* (P0 code baseline remains `889ebd35` + doc commit)  
**Purpose:** Factual inventory of who owns each Launcher axis today. Phase 11.2–11.5 applied **narrow write-gate convergence** on top of this map (see § Phase 11 convergence below).

**References:** `REFACTOR_PLAN.md` §6 P1.1–P1.5, `CONTRACTS.md` C-UI-001–004, `P0_EXIT_SUMMARY.md`, `PHASE11_LAUNCHER_REDUCTION_REPORT.md`.

---

## Phase 11 convergence (11.2–11.5, 2026-07-07)

| Slice | Status | Converged | Remaining risk |
|-------|--------|-----------|----------------|
| **11.2** RootController boundary | ✅ | Private write gates in `LauncherRootController` for query sync, selection, content mode transitions, panel-active snapshot apply; C-UI-004 clamp fallback in `LauncherListView.apply` | `lastSyncedQuery` still private dedup mirror; coordinator direct writes only via gates from RootController |
| **11.3** Detail lifecycle | ✅ | `applyDetailExitFromChrome` / `applyDetailExitOutcome`; `dismissDetailForNewQuery`; `persistDetailForActionDismiss`; shared `finalizePanelHidden()` | Hide still does not tear down detail (by design); `LauncherSessionState` detail events still partially unwired |
| **11.4** Task / MainActor (MVP) | ✅ | Clipboard MVP `@objc` search/copy/paste → `nonisolated` + `Task { @MainActor }` | 80+ warn-only `@objc` in parked/deferred detail views; Clipboard non-MVP handlers unchanged |
| **11.5** Cache vs repaint | ✅ | `LauncherHomeRefreshIntent` + `LauncherHomeRefreshRepaintPolicy`; background warm 不推进 `lastRenderedHomeGeneration`；generation 仅在 `showHome` 后更新 | `revalidateSnapshotInBackground()` still unused; menu-bar Show bypass (C-UI-001) open |

**P0 gate after 11.2–11.5:** `swift test` Launcher/Apps/Clipboard/Notes/Settings/DiagnosticsExport ✅; no new `.ips` (still 3 historical on 2026-07-06).

---

## Cross-cutting architecture

| Layer | Role |
|-------|------|
| `LauncherWindowController` | Panel show/hide, `LauncherPanelVisibilitySession`, defers UI to `LauncherRootView` |
| `LauncherRootController` | Orchestration hub: query dispatch, snapshot pipeline, detail lifecycle, home refresh, keyboard routing |
| `LauncherSessionState` (`LumaCore`) | Reducer for panel/content/detail axes — **only partially wired** in production |
| `LauncherContentCoordinator` | De-facto content mode + selection authority for in-panel UI |
| `LauncherViewModel` | Async query dispatch → `ResultSnapshot` production |
| MainActor | All launcher controllers are `@MainActor`; home providers (`LauncherHomeCoordinator`, `OpenAppsHomeProvider`) are `actor`s |

**P1.1 target:** Narrow `LauncherRootController` to wiring/dispatch; name explicit owners per axis.  
**P1.2 target:** One enforced owner per query/selection/mode/visibility; close C-UI deviations.

---

## 1. Launcher visibility (show / hide / panel visible)

| | |
|---|---|
| **Current owner** | `LauncherWindowController` + `LauncherPanelVisibilitySession` (`LauncherWindowController.swift`, `Sources/LumaCore/Home/LauncherPanelVisibilitySession.swift`) |
| **Canonical flag** | `visibilitySession.isVisible` → `isPanelVisible` |
| **Writers** | `beginShow()` / `beginHide()` inside `show()`, `hide()`, `hideImmediatelyForAction()`, `toggle()`, `showFromCarbonHotkey()`, `hideFromVisibleHotkey()` |
| **Readers** | Carbon hotkey guards; `AppCoordinator` external-activation hide; `repositionPanelIfVisible()` |
| **Multi-owner?** | **Partially reduced (11.5).** Background refresh no longer uses `panel.isVisible` during hide fade. Menu bar Show bypass remains (**C-UI-001**). |
| **MainActor / Task** | `@MainActor` class; hide animation uses `Task { @MainActor }`; space-change observer bridges via `Task` |
| **Tests** | `LauncherPanelVisibilitySessionTests`, `HotkeyDoubleFireTests`, `LauncherShowHideStateTests` |
| **P0 risk** | Rapid show/hide races (mitigated by generation tokens); menu-bar Show + hotkey sequencing; session vs `panel.isVisible` during fade |
| **Suggested owner (P1.2)** | Keep `LauncherWindowController` + session; route **all** show entry points through one guarded API (or prove menu-bar bypass safe with test) |

---

## 2. Query text

| | |
|---|---|
| **Current owner (UI SoT)** | `LumaSearchBar` — `textField.stringValue` / `queryText` |
| **Dispatch path** | `LauncherRootController.handleTextChange` → `LauncherViewModel.queryChanged` |
| **Writers** | NSTextField delegate; programmatic `stringValue`, `resetQueryText`, `appendText`; detail mode clears visible field, keeps `persistedQuery` |
| **Readers** | `QueryView(raw:)`; `viewModel.queryChanged`; snapshot apply via `isQueryEmpty` closure; permission banner (live field, C-UI-002) |
| **Multi-owner?** | **Partial.** `lastSyncedQuery` in `LauncherRootController` (dedup, not SoT). `launcherEnvironment.isLauncherQueryEmpty` mirror flag. Detail readers must use `persistedQuery`, not `stringValue` alone. |
| **MainActor / Task** | `LumaSearchBar` is AppKit (not `@MainActor`); `LauncherViewModel.queryChanged` uses unstructured `Task`, delivers on `MainActor.run` |
| **Tests** | `LauncherSearchDetailModeTests`, `LauncherSearchDetailModeAppWiringTests`; `QueryView` contract tests |
| **P0 risk** | IME composition + 200 ms poll lag; `LauncherFlowHarness` diverges from production wiring (C-TEST-004) |
| **Suggested owner (P1.2)** | Keep SearchBar → RootController → ViewModel path; extract IME/sync into named session helper |

---

## 3. Selected result

| | |
|---|---|
| **Current owner (interaction SoT)** | `LauncherListView.selectedFlatIndex` |
| **Bridge** | `LauncherContentCoordinator.selectedIndex` via `onSelectionChanged` |
| **Writers** | Mouse hover; keyboard via `LauncherKeyboardDispatcher` → `contentCoordinator.updateSelection`; snapshot apply preserves ID or falls back; `preferBareOpenDetailRowSelection` |
| **Readers** | `activateReturn` / `activateSelectedItem`; `LauncherKeyboardDispatcher` context; action panel when visible |
| **Multi-owner?** | **Partially reduced (11.2).** Write gates in `LauncherRootController` private extension route selection/content/query-empty transitions. **C-UI-004 mitigated (11.2):** missing ID → clamp prior flat index, not always 0. Coordinator remains de-facto SoT. |
| **MainActor / Task** | List key/mouse handlers: `nonisolated` → `Task { @MainActor }` |
| **Tests** | `LauncherListRowReuse`, `LauncherKeyRouterTests`; **gap:** stale selection after snapshot removal |
| **P0 risk** | Return runs wrong row after snapshot drops selected item |
| **Suggested owner (P1.2)** | `LauncherContentCoordinator` as authority; list as view binding; explicit “selection invalidated” instead of silent index-0 |

---

## 4. Content mode (home / results / detail)

| | |
|---|---|
| **Current owner** | `LauncherContentCoordinator.mode: LauncherContentMode` |
| **Type location** | `LauncherContentMode` in `Sources/LumaCore/Home/LauncherKeyRouter.swift` (**C-UI-003** doc says `LauncherContentCoordinator`) |
| **Writers** | `showHome`, `renderResults`, `present`, `closeDetail`, `tearDownDetailIfNeeded`, `dismissResultsForEmptyQuery`, `beginShowingResults`, `resetResults`; driven by `handleTextChange` |
| **Readers** | `showingDetail` / `showingResults`; `LauncherKeyboardDispatcher` infers mode from flags; split layout planner |
| **Multi-owner?** | **Yes — shadow reducer.** `LauncherSessionState.content` not kept in sync for most transitions. Keyboard dispatcher recomputes mode vs reading `contentCoordinator.mode`. |
| **MainActor / Task** | `@MainActor` coordinator |
| **Tests** | `LauncherContentModeTests` (wiring); `LauncherSessionStateTransitionTests` (reducer only) |
| **P0 risk** | Future code reading `LauncherSessionState` instead of coordinator → drift |
| **Suggested owner (P1.2)** | Coordinator remains SoT; resolve C-UI-003; optionally sync reducer from coordinator only |

---

## 5. Detail mode (search suspend + presentation)

| | |
|---|---|
| **Search-field detail mode** | `LumaSearchBar.detailModeState` + `LauncherSearchDetailMode` transitions |
| **Presentation mode** | `LauncherContentCoordinator` `.detail(moduleID)` via `present` / `closeDetail` |
| **Lifecycle** | `LauncherDetailPresenter` (enter/warmup), `LauncherDetailLifecycleController` (crossfade/close), `ModuleDetailRegistry` (pooling) |
| **Writers** | `beginDetailMode` / `endDetailMode` / `cancelDetailMode`; `exitDetailFromChrome`; `closeDetail` chain |
| **Readers** | `isDetailModeActive`, `persistedQuery`, `LauncherDetailExitPlanner`, `onDetailKey` |
| **Multi-owner?** | **Yes — four-way split** (registry / presenter / lifecycle / coordinator, P1.3). Two teardown strengths: pooled `closeDetail` vs hard `tearDownDetailIfNeeded`. |
| **MainActor / Task** | `LauncherDetailPresenter.openModuleDetail` uses `Task { @MainActor }` |
| **Tests** | `LauncherSearchDetailModeTests`, `DetailHierarchyReuseTests`, `LauncherDetailExitPlannerTests` |
| **P0 risk** | Stuck detail mode if teardown skips `endDetailMode` (`clearStuckDetailModeState`) |
| **Suggested owner (P1.3)** | Document four layers; wire `LauncherSessionState.detailMode` fully or drop duplicate axis |

---

## 6. Home snapshot

| | |
|---|---|
| **Current owner** | `LauncherHomeCoordinator` (actor) — `cachedSnapshot`, `snapshotGeneration` |
| **Providers** | `LauncherHomeAggregator` + `OpenAppsHomeProvider` |
| **Writers** | `snapshot(forceRefresh:)`; `invalidateSnapshotCache`; `OpenAppsHomeProvider` refresh when `isActive` |
| **Readers** | `LauncherRootController.refreshHome`; `LauncherDetailPresenter` (cache for split layout); `LauncherContentCoordinator.showHome` |
| **Multi-owner?** | **Partial.** `lastRenderedHomeGeneration` in `LauncherRootController`. `contentCoordinator.currentItems` mirrors home flat items. |
| **MainActor / Task** | `refreshHome` Task: `await homeCoordinator.snapshot()` → `MainActor.run`; `setPanelSignalsActive` via `Task` |
| **Tests** | Implicit via latency/home paths; **gap:** isolated home-coordinator unit tests |
| **P0 risk** | Stale cache short-circuit by generation — intentional perf path |
| **Suggested owner (P1.2)** | `LauncherHomeCoordinator` stays owner; RootController only triggers refresh |

---

## 7. Module snapshot (`ResultSnapshot`)

| | |
|---|---|
| **Production owner** | `LauncherViewModel` — dispatch sequence → `QueryDispatcher` |
| **Apply owner** | `LauncherSnapshotApplyPipeline` → `LauncherContentCoordinator.apply` |
| **Coalescing** | `LauncherSnapshotApplyCoalescer` (16 ms frame cadence) |
| **Writers** | `viewModel.queryChanged`; workbench preview local snapshot; help/suggestion in-VM snapshots |
| **Readers** | `enqueueSnapshotApply`; `LauncherSnapshotApplyPolicy` (`isPanelActive`, `isQueryEmpty`) |
| **Multi-owner?** | **Split produce/consume.** `isPanelActiveForQueryApply` in RootController separate from visibility session. |
| **MainActor / Task** | Dispatch in unstructured `Task`; delivery `MainActor.run`; coalescer `Task` on MainActor |
| **Tests** | `LauncherSnapshotApplyCoalesceTests`, `HideDuringSnapshotApplyTests`, `LauncherSnapshotApplyPolicyTests`, `WorkbenchPreviewHideRaceTests` |
| **P0 risk** | Stale apply after hide (mitigated by policy + `cancelPending` on hide) |
| **Suggested owner (P1.2)** | ViewModel produce / pipeline apply; tie `isPanelActiveForQueryApply` to visibility generation |

---

## 8. Status / toast

| | |
|---|---|
| **UI owner** | `CommandHintBar.showStatus` (auto-dismiss Task) |
| **Local facade** | `LauncherRootController.showStatus` |
| **Global facade** | `LauncherEnvironment.showStatus` → `AppCoordinator` → `windowController.showStatus` |
| **Writers** | Many call sites in RootController, module callbacks, action outcomes |
| **Readers** | `statusLine` label only |
| **Multi-owner?** | **Many writers, two entry APIs** — same UI target, no central queue |
| **MainActor / Task** | Dismiss `Task` with `MainActor.run` |
| **Tests** | Indirect; **gap:** dedicated status-owner test |
| **P0 risk** | Low |
| **Suggested owner (P1.2)** | Single `StatusPresenter`; environment holds weak ref |

---

## 9. Latency tracker

| | |
|---|---|
| **Keystroke → paint** | `LatencyTracker.shared` in `LumaSearchBar` / `markFirstPaint` in RootController on snapshot apply |
| **Hotkey → home** | `HomeLatencyTracker` — `markHotkey` on show, `markHomeRendered` after home paint, `abandonPendingHotkeyMark` on hide |
| **Dispatch p95** | `LauncherViewModel.latencySamples` — internal, not HUD |
| **Writers** | Show/hide/snapshot/home render paths |
| **Readers** | `LauncherPerformanceStripView` / latency HUD |
| **Multi-owner?** | **Three parallel trackers** with different lifetimes |
| **MainActor / Task** | `@MainActor` trackers |
| **Tests** | **Gap:** dedicated unit tests; QA `LUMA_QA` export path |
| **P0 risk** | Stale hotkey mark if home render skipped — mitigated by `abandonPendingHotkeyMark` |
| **Suggested owner (P1.5)** | Consolidate behind `LauncherLatencyRecorder` |

---

## 10. Background refresh

| | |
|---|---|
| **Current owner** | `OpenAppsHomeProvider` refresh loop when `isActive`; orchestrated by `LauncherHomeCoordinator.setActive` |
| **Triggers** | Panel show/hide → `setPanelSignalsActive`; `onCacheUpdated` → `refreshHomeForBackgroundDataUpdate`; wake notification; external app activation |
| **Writers** | Provider loop; RootController `refreshHome`; WindowController visibility gates |
| **Readers** | Home coordinator cache; list render via `showHome` |
| **Multi-owner?** | **Partially reduced (11.5).** `refreshHomeForBackgroundDataUpdate` now gates on `visibilitySession.isVisible` and uses `LauncherHomeRefreshIntent.backgroundCacheWarm` (no list paint / latency). `revalidateSnapshotInBackground()` **unused** (no call sites). |
| **MainActor / Task** | `OpenAppsHomeProvider` is `actor`; callbacks `Task { @MainActor }` |
| **Tests** | **Gap:** none specific |
| **P0 risk** | Open Apps cache stale while panel visible (refresh suppressed) |
| **Suggested owner (P1.5)** | `LauncherHomeCoordinator` owns policy; single `scheduleHomeRefresh(reason:)` |

---

## 11. Keyboard event routing

| | |
|---|---|
| **Entry points** | `LauncherPanel` Esc / ⌘Space / ⌘W; `LumaSearchBar` / `LauncherListView`; `LauncherActionPanel` |
| **Routing** | `LauncherRootController.handleKeyCommand` → `LauncherKeyboardDispatcher` → `LauncherKeyRouter` |
| **Detail keys** | `dispatchDetailKeyDown`, `searchBar.onDetailKey`, `panel.onDetailKeyDown` |
| **Escape** | `LauncherEscapePlanner` + `handleEscape` |
| **Multi-owner?** | Mode inferred in dispatcher vs coordinator (see §4). AppKit `nonisolated` → `Task { @MainActor }` (P1.4 class). |
| **MainActor / Task** | Multiple AppKit boundary bridges |
| **Tests** | `LauncherKeyRouterTests`, `LauncherEscapePlannerTests`, `DetailTypingEscapeConsistencyTests` |
| **P0 risk** | AppKit executor crashes; ↑↓ blocked in detail (by design) |
| **Suggested owner (P1.2)** | `LauncherKeyboardDispatcher` stays router; RootController stays wiring hub |

---

## 12. Show / hide entry points

| Entry | Path | Guards |
|-------|------|--------|
| Carbon global hotkey | `HotkeyController` → `showFromCarbonHotkey()` | Hidden-only + 120 ms debounce |
| Visible ⌘Space | `LauncherPanel.performKeyEquivalent` → `hideFromVisibleHotkey()` | Visible-only + debounce |
| Toggle | `LauncherWindowController.toggle()` | Debounce; uses session `isVisible` |
| Menu bar “Show Luma” | `MenuBarController` → `windowController.show()` | **No hidden-only guard; no Carbon debounce (C-UI-001)** |
| Esc dismiss | Escape planner → `hide()` | Chain |
| External app activation | `hideIfShowingForExternalActivation` | Skips self-bundle |
| Action dismiss | `hideImmediatelyForAction` | Skips fade |
| Settings open | `hideImmediatelyForAction` from `AppCoordinator` | — |

**Tests:** `HotkeyDoubleFireTests`, `LauncherShowHideStateTests`  
**P0 risk:** Menu-bar Show + hotkey inconsistent state  
**Suggested owner (P1.2):** Unified `show(reason:)` with per-reason policy

---

## `LauncherSessionState` — production wiring

| Event | Wired? | Call site |
|-------|--------|-----------|
| `panelShowCompleted` | ✅ | `activatePanelForQueryApply` |
| `panelHideBegan` | ✅ | `cancelActiveQueryAndSnapshotApply` |
| `detailOpened` | ✅ | `LauncherDetailPresenter` |
| `detailClosed` | ✅ | `closeDetail` tearDown |
| `panelShowBegan`, `panelHideCompleted`, `queryBecame*`, `detailOpenRequested`, `detailExitRequested`, `userTypedInDetail` | ❌ | Tests only (`LauncherSessionStateTransitionTests`) |

**Implication:** Shadow reducer is **not** the enforced session owner; coordinator + window controller remain de-facto SoT.

---

## P1 target ownership summary

| Domain | P1.2+ target |
|--------|----------------|
| Visibility | `LauncherWindowController` + unified show API |
| Query | `LumaSearchBar` (UI) + `LauncherViewModel` (dispatch) |
| Selection | `LauncherContentCoordinator` ← `LauncherListView` binding |
| Content mode | `LauncherContentCoordinator` |
| Detail mode | Four-layer split (P1.3); search suspend in `LumaSearchBar` |
| Home snapshot | `LauncherHomeCoordinator` |
| Module snapshot | `LauncherViewModel` / `LauncherSnapshotApplyPipeline` |
| Status | Central presenter on RootController |
| Latency | Single recorder (P1.5) |
| Background refresh | `LauncherHomeCoordinator` (P1.5) |
| Keyboard | `LauncherKeyboardDispatcher` + `LauncherKeyRouter` |
| Show/hide entry | `LauncherWindowController` visibility API |

---

## Test coverage gaps (P1.2 acceptance targets)

- Menu bar Show + rapid hotkey sequencing (C-UI-001)
- Return after snapshot removes selected item (C-UI-004) — **partial:** clamp fallback added; full integration test still gap
- Return / Esc / hide / re-summon selection + mode sequence
- `LauncherSessionState` production wiring parity
- Isolated `LauncherHomeCoordinator` / background refresh policy

---

## Regression gate reminder

Any P1 slice must pass **P0 MVP Smoke Gate** (`docs/QA.md` § P0 MVP Smoke Gate) before merge. New `.ips` → stop and triage P0.

---

*Phase 11.1 — initial map. Phase 11.2–11.5 — convergence per table above.*
