# Phase 11 Launcher Reduction Report (11.2–11.5)

**Date:** 2026-07-07  
**Baseline commit:** `803b0672` — *Document P0 smoke gate and phase 9 exit*  
**P0 code baseline:** `889ebd35` — *Add QA smoke hooks and config corruption tracking*

---

## Phase 11 总结

### 1. Baseline commit

`803b0672` (docs on top of P0 code `889ebd35`). Work is **uncommitted** on `main`.

### 2. 改动文件列表

| File | Slice | Change |
|------|-------|--------|
| `Sources/LumaApp/Launcher/LauncherRootController.swift` | A, B, D | Write gates; detail exit helpers; `refreshHome(intent:)` |
| `Sources/LumaApp/Launcher/LauncherListView.swift` | A | C-UI-004 clamp fallback on snapshot selection |
| `Sources/LumaApp/Launcher/LauncherWindowController.swift` | B, D | `finalizePanelHidden()`; visibility-session background refresh |
| `Sources/LumaApp/Launcher/LauncherRootView.swift` | D | `refreshHome(intent:)` forwarding |
| `Sources/LumaCore/Home/LauncherHomeRefreshIntent.swift` | D | **New** — intent + `LauncherHomeRefreshRepaintPolicy` pure logic |
| `Sources/LumaApp/Launcher/ClipboardDetailView.swift` | C | MVP search/copy/paste `@objc` → `nonisolated` |
| `Tests/LumaAppTests/LauncherDetailLifecycleBoundaryTests.swift` | B | **New** — wiring tests |
| `Tests/LumaCoreTests/LauncherHomeRefreshRepaintPolicyTests.swift` | D | **New** — behavioral policy tests |
| `LAUNCHER_STATE_OWNER_MAP.md` | docs | 11.2–11.5 convergence markers |
| `REFACTOR_PLAN.md` | docs | Phase 11 status table |

### 3. Slice A — Phase 11.2 LauncherRootController Boundary

**收敛写入入口：**

- Query: `recordQueryTextChange`, `resetSyncedQueryForRestore`, `syncQueryBaselineFromSearchField`, `recordQueryEmptyState`
- Selection: `setSelectionIndex`, `moveSelection`
- Content mode: `transitionToEmptyQueryHome`, `transitionToResultsMode`, `transitionToClearedResultsList`
- Panel-active snapshot apply: `markPanelInactiveForSnapshotApply`, `markPanelActiveForSnapshotApply`
- Selection fallback: `LauncherListView.apply` clamps flat index when preserved ID missing (C-UI-004 mitigation)

**仍保留到后续 slice：**

- `LauncherContentCoordinator` 内部 mode/selection 仍是 UI SoT
- `LumaSearchBar` detail mode 直接写入（P1.3 四层拆分未做）
- `LauncherSessionState` 大部分 event 仍 test-only（P1.2）
- `showHome` / `restoreHomeFromDetail` 仍直接调用 coordinator teardown（未进一步抽取）

### 4. Slice B — Phase 11.3 Detail Lifecycle Boundary

**detail lifecycle 入口现状：**

| Entry | Handler |
|-------|---------|
| Esc / back / close chrome | `exitDetailFromChrome` → `applyDetailExitFromChrome` → `applyDetailExitOutcome` |
| User types new query | `dismissDetailForNewQuery` |
| Keyboard close | `dispatchDetailCloseFromKeyboard` → `exitDetailFromChrome` |
| Panel hide (fade) | `cancelActiveQueryAndSnapshotApply` + `prepareDetailForHide` → `finishHide` → `saveCurrentSession` + `finalizePanelHidden` |
| Action dismiss hide | `hideImmediatelyForAction` → `prepareDetailForHide` → `resetForActionDismiss` / `persistDetailForActionDismiss` + `finalizePanelHidden` |

**已收敛：** 重复 panel order-out 清理合并为 `finalizePanelHidden()`；action dismiss 与 chrome exit 命名边界清晰。

**未收敛风险：** Hide 仍不 tear down detail（设计如此）；Clipboard/Notes detail 业务逻辑未改。

### 5. Slice C — Phase 11.4 Task / MainActor Cleanup

**修了：** `ClipboardDetailView` MVP 路径 `searchChanged`, `copySelected`, `copySelectedPlainText`, `pasteSelected`, `doubleClickRow` → `nonisolated` + MainActor hop.

**留到 P2：** Snippets/Wordbook/Translate/Todo/Media/Secrets/Quicklinks/Projects detail `@objc` warns；Clipboard 非 MVP handlers（filter, clear, transform）。

**Scanner：** `bash scripts/scan_appkit_executor_risk.sh` → **OK: no AppKit executor risks detected** (warn-only reduced for Clipboard MVP).

### 6. Slice D — Phase 11.5 Cache Refresh vs UI Repaint

**分离完成：**

- `LauncherHomeRefreshIntent.backgroundCacheWarm` — 仅拉 snapshot，不 `showHome`、不写 latency
- `visibleRepaint` — 原有 paint 路径；`HomeLatencyTracker.markHomeRendered` 仅在 `isPanelActiveForQueryApply`
- `refreshHomeForBackgroundDataUpdate` 改用 `!visibilitySession.isVisible`（非 `panel.isVisible`）
- `lastRenderedHomeGeneration` **仅在** `contentCoordinator.showHome(...)` 实际执行后推进（修复 cache/UI 混同）

**测试护栏：**

- **行为：** `LauncherHomeRefreshRepaintPolicyTests`（LumaCore）— repaint / generation / latency 纯逻辑
- **接线：** `LauncherHomeRefreshPolicyTests`、`LauncherDetailLifecycleBoundaryTests` — 源码字符串，**不算强行为护栏**

**Latency：** Phase 9 基线 hotkey p95 ~28 ms、keystroke p95 ~20 ms；本 slice 未引入新 latency 关闭点污染（abandon + panel-active guard）。

### 7. 每个 slice 的测试结果

| Slice | Tests |
|-------|-------|
| A | `swift test --filter Launcher` — **123 passed** |
| B | Launcher **123** + Clipboard **56** + Notes **49** — all passed |
| C | + Settings **2** + DiagnosticsExport **5** — all passed |
| D | + `LauncherHomeRefreshRepaintPolicy` **7** + `LauncherHomeRefreshPolicy` **2** + AppsModuleTests **4** — all passed |
| Final | Full filtered suite above — **all passed** |

### 8. P0 gate 是否仍通过

**Yes** (automated gate):

- `swift build` ✅
- Launcher / AppsModuleTests / Clipboard / Notes / Settings / DiagnosticsExport ✅
- No new `~/Library/Logs/DiagnosticReports/Luma-*.ips`

**Signed-app smokes:** `build/Luma.app` rebuilt ✅; `LUMA_QA_*` hooks are non-terminating (export runs async). Phase 9.8 smokes remain the signed-app baseline; not re-validated end-to-end in this session.

### 9. `.ips` 前后对比

| When | Count | Notes |
|------|-------|-------|
| Before (Phase 9) | 3 | `Luma-2026-07-06-{115651,165734,184548}.ips` |
| After Phase 11 | 3 | **No new crashes** |

### 10. 是否修改 MVP scope

**No.** No new modules, no parked/deferred default changes, no QueryDispatcher/ModuleHost rewrites.

### 11. 是否触碰 parked/deferred 模块

**No product semantic changes.** Scanner warn-only touches parked detail views only in documentation classification; code changes limited to P0 MVP surfaces.

### 12. 剩余 P1/P2/P3 backlog

| Item | Status |
|------|--------|
| P1.2 Session state owner (`LauncherSessionState` wiring) | Not started |
| C-UI-001 menu-bar Show bypass | Open |
| C-UI-003 `LauncherContentMode` doc location | Open |
| C-UI-004 full Return-after-snapshot test | Partial (clamp only) |
| P1.4 remaining `@objc` in non-MVP details | P2 |
| `revalidateSnapshotInBackground()` wiring | P2 |
| `LauncherFlowHarness` vs production parity (C-TEST-004) | P2 |

### 13. 下一阶段建议

1. **P1.2** — Wire `LauncherSessionState` production events or formally deprecate shadow reducer.
2. **C-UI-001** — Unified `show(reason:)` with menu-bar policy test.
3. **C-UI-004** — Integration test: snapshot removes selected row → Return does not run wrong item.
4. Optional: terminable `LUMA_QA_*=1` smoke runner for CI signed-app gate.

---

*Phase 11 combined execution complete. Stop-on-red not triggered.*
