# P1 Exit Summary

**Date:** 2026-07-07  
**P0 baseline commit:** `889ebd35` — *Add QA smoke hooks and config corruption tracking*  
**P1 work commit:** `c53fb635` — *Refine launcher session and show-entry governance* (Phase 11–12)  
**Phase 13 commit readiness:** `LauncherWindowController` policy consistency fix (pending commit)  
**P1 Exit verdict:** **Go** ✅

---

## 1. Baseline commit

| Milestone | Commit | Notes |
|-----------|--------|-------|
| P0 code + smoke hooks | `889ebd35` | Frozen MVP functional baseline |
| P0 exit docs | `803b0672` | `docs/QA.md` smoke gate |
| P1 Launcher reduction | `c53fb635` | Phase 11–12 combined |

---

## 2. Phase 11 完成内容 (11.2–11.5)

| Slice | Deliverable |
|-------|-------------|
| **11.2** | `LauncherRootController` private write gates (query, selection, content mode, panel-active snapshot apply); C-UI-004 selection clamp in `LauncherListView` |
| **11.3** | Detail exit helpers (`applyDetailExitFromChrome`, `dismissDetailForNewQuery`, `persistDetailForActionDismiss`); `finalizePanelHidden()` |
| **11.4** | Clipboard MVP `@objc` → `nonisolated` + MainActor hop (search/copy/paste) |
| **11.5** | `LauncherHomeRefreshIntent` + `LauncherHomeRefreshRepaintPolicy`; background vs visible repaint split; `lastRenderedHomeGeneration` only after real `showHome` |

**Report:** `PHASE11_LAUNCHER_REDUCTION_REPORT.md`

---

## 3. Phase 12 完成内容

| Slice | Deliverable |
|-------|-------------|
| **12.1** | `LAUNCHER_STATE_AUDIT.md` — session state facts; recommend test-only reducer |
| **12.2** | `LAUNCHER_SHOW_ENTRY_CONTRACT.md` — show/hide entry table + intentional differences |
| **12.3** | `show(reason:)` / `showFromMenuBar()`; `LauncherShowEntryPolicy`; menu bar behavioral tests |
| **12.4** | `LauncherListSelectionPreservePolicy` + `LauncherReturnActivationPolicy`; C-UI-004 behavioral tests |

**Report:** `PHASE12_SESSION_SHOW_GOVERNANCE_REPORT.md`

---

## 4. Phase 13 — commit readiness

| Task | Result |
|------|--------|
| Review cleanup | `show(reason:)` uses `LauncherShowEntryPolicy.appliesCarbonShowDebounce(reason:)` (no inline `reason == .carbonHotkey`) |
| Documentation pass | Phase 11/12 reports + `REFACTOR_PLAN.md` updated; this file added |
| Regression gate | `swift test` 792/792 ✅ |
| Signed app | `./scripts/build_app.sh` ✅ |

---

## 5. 改动文件总表

### Source (`Sources/`)

| File | Phase |
|------|-------|
| `LumaApp/Launcher/LauncherRootController.swift` | 11, 12 |
| `LumaApp/Launcher/LauncherWindowController.swift` | 11, 12, **13** |
| `LumaApp/Launcher/LauncherRootView.swift` | 11 |
| `LumaApp/Launcher/LauncherListView.swift` | 11, 12 |
| `LumaApp/Launcher/ClipboardDetailView.swift` | 11 |
| `LumaApp/App/AppCoordinator.swift` | 12 |
| `LumaCore/Home/LauncherHomeRefreshIntent.swift` | 11 (includes `LauncherHomeRefreshRepaintPolicy`) |
| `LumaCore/Home/LauncherShowEntryPolicy.swift` | 12 |
| `LumaCore/Home/LauncherListSelectionPreservePolicy.swift` | 12 |

### Tests (`Tests/`)

| File | Phase |
|------|-------|
| `LumaLinterTests/ModuleDisableWiringTests.swift` | 12 |
| `LumaAppTests/LauncherDetailLifecycleBoundaryTests.swift` | 11 |
| `LumaAppTests/LauncherHomeRefreshPolicyTests.swift` | 11 |
| `LumaCoreTests/LauncherHomeRefreshRepaintPolicyTests.swift` | 11 |
| `LumaAppTests/LauncherMenuBarShowEntryTests.swift` | 12 |
| `LumaCoreTests/LauncherShowEntryPolicyTests.swift` | 12 |
| `LumaCoreTests/LauncherSnapshotSelectionPolicyTests.swift` | 12 |

### Documentation

| File | Phase |
|------|-------|
| `LAUNCHER_STATE_OWNER_MAP.md` | 11.1 + 11 convergence markers |
| `LAUNCHER_STATE_AUDIT.md` | 12.1 |
| `LAUNCHER_SHOW_ENTRY_CONTRACT.md` | 12.2 |
| `PHASE11_LAUNCHER_REDUCTION_REPORT.md` | 11 |
| `PHASE12_SESSION_SHOW_GOVERNANCE_REPORT.md` | 12 |
| `REFACTOR_PLAN.md` | 11–13 status |
| `P1_EXIT_SUMMARY.md` | 13 (this file) |

---

## 6. 测试结果

| Suite | Count | Result |
|-------|-------|--------|
| `swift build` | — | ✅ |
| `swift test` (full) | **792** | ✅ |
| `--filter Launcher` | 142 | ✅ |
| `--filter LauncherHomeRefresh` | 9 | ✅ |
| `--filter LauncherShowEntryPolicy` | 3 | ✅ |
| `--filter LauncherSnapshotSelectionPolicy` | 6 | ✅ |
| `--filter LauncherAction` | 13 | ✅ |
| `--filter Clipboard` | 56 | ✅ |
| `--filter Notes` | 49 | ✅ |
| `--filter Settings` | 2 | ✅ |
| `--filter DiagnosticsExport` | 5 | ✅ |

---

## 7. Signed app / smoke 结果

| Check | Result |
|-------|--------|
| `./scripts/build_app.sh` | ✅ `build/Luma.app` signed |
| `LUMA_QA_APPS=1` | ✅ `~/Library/Logs/Luma/apps-smoke.json` (`launchResult: success`) |
| `LUMA_QA_CLIPBOARD=1` | ✅ `clipboard-smoke.json` (`copySucceeded: true`) |
| `LUMA_QA_NOTES=1` | ✅ `notes-smoke.json` |
| `LUMA_QA_SETTINGS=1` | ✅ `settings-smoke.json` |
| `LUMA_QA_EXPORT=1` | ⚠️ Not validated end-to-end (async export, non-terminating app); Phase 9.3 baseline stands |
| Menu bar / hotkey manual soak | Deferred — automated policy + `HotkeyDoubleFireTests` cover guards |

**Latency** (from existing `~/Library/Logs/Luma/latency-report.json`):

| Metric | Value | P0 ceiling |
|--------|-------|------------|
| hotkey p95 | ~28 ms | ≤ 1 s ✅ |
| keystroke p95 | ~20 ms | ≤ 120 ms ✅ |

---

## 8. `.ips` 前后对比

| When | Count |
|------|-------|
| Before Phase 13 smokes | 17 (historical) |
| After Phase 13 smokes | **17** (no new) |

---

## 9. P0 gate 是否仍通过

**Yes.** No new crashes; latency within P0 ceilings; MVP module smokes green; full unit suite green.

---

## 10. MVP scope 是否变化

**No.** No new modules, no QueryDispatcher/ModuleHost/ranking changes, no parked/deferred default changes.

---

## 11. parked / deferred 模块是否变化

**No** product semantic or default-state changes. Non-MVP detail `@objc` warn-only items remain (P2).

---

## 12. 剩余 backlog

| Priority | Item |
|----------|------|
| **P2** | Delete or promote `LauncherSessionState` (see `LAUNCHER_STATE_AUDIT.md`) |
| **P2** | Non-MVP AppKit `@objc` warn cleanup (parked detail views) |
| **P2** | Clipboard / Notes UX polish |
| **P3** | Docs governance (`C-UI-003` type location, ENGINEERING.md show paths) |
| **P3** | `LauncherFlowHarness` ↔ production wiring alignment (C-TEST-004) |
| **P3** | C-UI-004 end-to-end integration test via production path |
| **P3** | Terminable `LUMA_QA_*` smoke runner for CI |

---

## 13. 是否建议 commit

**Yes.**

**Suggested commit scope:**

1. **If Phase 13 fix not yet committed:** single commit on top of `c53fb635`:
   - `LauncherWindowController.swift` — `appliesCarbonShowDebounce` policy call
   - `P1_EXIT_SUMMARY.md`, updated `PHASE11_*` / `PHASE12_*` / `REFACTOR_PLAN.md`

2. **If squashing P1:** one commit from `803b0672` containing all Phase 11–13 artifacts (already largely in `c53fb635` + Phase 13 delta).

**Do not include** unrelated untracked planning docs from other phases unless explicitly requested.

---

## Regression rule (unchanged from P0)

Any new `Luma-*.ips`, hotkey p95 > 1 s, keystroke p95 > 120 ms, or MVP smoke failure → stop P2 work and return to P0 triage per `P0_EXIT_SUMMARY.md`.

---

*Phase 13 — P1 Exit / Commit Readiness complete.*
