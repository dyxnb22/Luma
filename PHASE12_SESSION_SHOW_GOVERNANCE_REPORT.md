# Phase 12 — Session State & Show Entry Governance Report

**Date:** 2026-07-07  
**Baseline:** `803b0672` + Phase 11–12 uncommitted work

---

## 改动文件列表

### 文档（12.1–12.2）
- `LAUNCHER_SESSION_STATE_AUDIT.md` — **new**
- `LAUNCHER_SHOW_ENTRY_CONTRACT.md` — **new**
- `REFACTOR_PLAN.md` — Phase 12 table

### 源码（12.3–12.4）
- `Sources/LumaCore/Home/LauncherShowEntryPolicy.swift` — **new** (`LauncherShowReason`, show guards)
- `Sources/LumaCore/Home/LauncherListSelectionPreservePolicy.swift` — **new** (C-UI-004 + Return policy)
- `Sources/LumaApp/Launcher/LauncherWindowController.swift` — `show(reason:)`, `showFromMenuBar()`
- `Sources/LumaApp/App/AppCoordinator.swift` — menu bar → `showFromMenuBar()`
- `Sources/LumaApp/Launcher/LauncherListView.swift` — uses selection policy
- `Sources/LumaApp/Launcher/LauncherRootController.swift` — Return policy + session audit comment
- `Tests/LumaLinterTests/ModuleDisableWiringTests.swift` — fix for Phase 11 `markPanelInactive` helper

### 测试
- `Tests/LumaCoreTests/LauncherShowEntryPolicyTests.swift` — **new** (3 behavioral)
- `Tests/LumaCoreTests/LauncherSnapshotSelectionPolicyTests.swift` — **new** (6 behavioral, C-UI-004)
- `Tests/LumaAppTests/LauncherMenuBarShowEntryTests.swift` — **new** (2 behavioral + 1 wiring)

---

## 是否修改源码

**Yes** — minimal, scoped to show entry naming/policy and selection/Return pure logic extraction. No QueryDispatcher, ModuleHost, parked modules, or MVP scope changes.

---

## Session state 最终处理建议

**Keep test-only specification; do not wire remaining 7 events in Phase 12.**

| Recommendation | Rationale |
|----------------|-----------|
| **Retain** `LauncherSessionState` + transition tests | Documents illegal transitions; cheap spec |
| **Keep** 4 wired production events as legacy side-effect hooks | `panelHideBegan` → cancel tasks; `panelShowCompleted` → snapshot apply; `detailOpened` / `detailClosed` |
| **Do not wire** `panelShowBegan`, `panelHideCompleted`, query events, `detailExitRequested`, `userTypedInDetail` | Duplicates real owners; `panelHideCompleted` gap would leave shadow stuck in `.hiding` |
| **P2 option** | Delete reducer after migrating effects to explicit calls, or promote `visibilitySession` to single panel axis |

---

## Show entry 语义是否统一

**Partially unified — documented + named, not fully merged.**

| Before | After |
|--------|-------|
| Menu bar called raw `show()` | `showFromMenuBar()` → `show(reason: .menuBar)` |
| Carbon used ad-hoc guard | `show(reason: .carbonHotkey)` + `LauncherShowEntryPolicy` |
| Intentional bypass | **Documented** in `LAUNCHER_SHOW_ENTRY_CONTRACT.md` as refocus-when-visible |

User-visible behavior unchanged: menu bar still re-fronts when visible; carbon still hidden-only.

---

## 新增测试覆盖

| Area | Tests | Type |
|------|-------|------|
| Show entry policy | `LauncherShowEntryPolicyTests` (3) | **Behavioral** |
| Menu bar visible Show | `LauncherMenuBarShowEntryTests` (3) | **Behavioral** + wiring |
| C-UI-004 selection clamp | `LauncherSnapshotSelectionPolicyTests` (6) | **Behavioral** |
| Return stale index guard | same file | **Behavioral** |
| Detail lifecycle | `LauncherDetailLifecycleBoundaryTests` | Wiring-only (unchanged) |

**Harness limit (12.4):** Selection/Return policies are tested as pure logic. Full `LauncherRootController` → snapshot apply → Return chain is **not** covered by `LauncherFlowHarness` (C-TEST-004 divergence). Integration gap documented in test file comments.

---

## P0 gate

| Check | Result |
|-------|--------|
| `swift build` | ✅ |
| `swift test --filter LauncherHomeRefresh` | ✅ 9 |
| `swift test --filter Launcher` | ✅ 142 |
| `swift test --filter LauncherAction` | ✅ 13 |
| `swift test` (full) | ✅ 792 |
| New `.ips` | **None found** in `~/Library/Logs/DiagnosticReports/` at audit time |
| Signed-app hotkey/menu smoke | Not re-run (non-terminating QA hooks) |

---

## Phase 13 建议入口

1. **P2:** Decide delete vs promote `LauncherSessionState` after effect migration plan
2. **C-UI-004:** Optional harness-level integration test if `LauncherFlowHarness` gains `LauncherRootController` path (P3.2)
3. **C-UI-001:** Optional `show(reason: .restore)` when session restore gets explicit entry
4. **Latency:** Terminable `LUMA_QA_*` runner for signed-app CI gate

---

*Phase 12 complete. No P0 regression observed.*
