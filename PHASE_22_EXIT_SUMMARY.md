# Phase 22 Exit Summary — Launcher E2E Usability Governance

**Date:** 2026-07-07  
**Verdict:** Phase 22 **complete** (observability + fixes + keyboard smoke). **RC candidacy:** **No-Go** (RC step 9 manual supplement + UR manual re-test still required).

---

## 1. Discovered state fractures

| ID | Fracture | Evidence | Fix |
|----|----------|----------|-----|
| F-01 | `enterDetailContext` before `contentCoordinator.present` → search detail placeholder while split still on guide | UR-001, invariant I1 | `LauncherDetailPresenter`: stage detail context inside `present` block |
| F-02 | `handleEscape` / split planner used `showingDetail` only; empty `stringValue` in detail mode → panel hide | UR-002, I1 | `detailContextActive` on `LauncherEscapePlanner`; unified detail signal in `currentHomeSplitState()` |
| F-03 | Hide/reopen left `isDetailModeActive` XOR `showingDetail` | UR-003, I5 | `reconcileLauncherStateAfterShow()` on panel show |
| F-04 | Animation cancel without layout settle | I6 risk | `syncSplitLayout()` after `cancelLauncherAsyncWork` (already present) + invariant checker |

---

## 2. Deliverables

| Slice | Artifact |
|-------|----------|
| 22.0 | [`USABILITY_REGRESSION_2026_07_07.md`](USABILITY_REGRESSION_2026_07_07.md), [`RC_BLOCKERS.md`](RC_BLOCKERS.md) |
| 22.1 | [`Sources/LumaCore/Launcher/LauncherStateSnapshot.swift`](Sources/LumaCore/Launcher/LauncherStateSnapshot.swift), collector/exporter, `~/Library/Logs/Luma/launcher-state.json` |
| 22.2 | [`LAUNCHER_KEYBOARD_FLOW_MATRIX.md`](LAUNCHER_KEYBOARD_FLOW_MATRIX.md) |
| 22.3 | [`scripts/qa/run_keyboard_flows.sh`](scripts/qa/run_keyboard_flows.sh), `drive.sh` extensions (`menu-show`, `cmd-space`, `wait-state`, `focus`) |
| 22.4 | `LauncherStateInvariantChecker` I1–I6, `launcher-state-violations.json` |
| 22.5 | Presenter, escape planner, root controller call sites, window show reconcile |

---

## 3. Fix file list

- `Sources/LumaApp/Launcher/Session/LauncherDetailPresenter.swift`
- `Sources/LumaCore/Home/LauncherEscapePlanner.swift`
- `Sources/LumaApp/Launcher/LauncherRootController.swift`
- `Sources/LumaApp/Launcher/LauncherWindowController.swift`
- `Sources/LumaApp/Launcher/LauncherHomeSplitLayout.swift`
- `Sources/LumaApp/Launcher/LauncherHintBar.swift`
- `Sources/LumaApp/Launcher/LumaSearchBar.swift`
- `Sources/LumaApp/Launcher/LauncherStateSnapshotCollector.swift`
- `Sources/LumaApp/Infrastructure/LauncherStateSnapshotExporter.swift`
- `Sources/LumaCore/Launcher/LauncherStateSnapshot.swift`
- `Sources/LumaCore/Home/LauncherPanelVisibilitySession.swift`
- `docs/QA.md`

---

## 4. Automation flows

| Flow | Script | Status (2026-07-07) |
|------|--------|---------------------|
| KF-01 open → Esc | `run_keyboard_flows.sh` | PASS |
| KF-02 Notes `n` detail → Esc | same | PASS |
| KF-03 Clipboard `cb` results (+ Return) | same | PASS (results; full detail open via `ClipboardProductionSmoke`) |
| KF-04 hide → reopen | same | PASS |
| KF-05 menu Show → query → Esc | same | PASS |

---

## 5. Gate results

| Check | Result |
|-------|--------|
| `swift build` | PASS |
| `swift test --filter Launcher` | PASS (149) |
| `swift test --filter LauncherDetail` | PASS (6) |
| `swift test --filter LauncherStateInvariant` | PASS (5) |
| `./scripts/build_app.sh --no-restart` | PASS |
| `./scripts/qa/run_keyboard_flows.sh` | PASS (failures 0, ips delta 0) |

---

## 6. RC candidacy

**No-Go** until:

1. Human re-test of UR-001–UR-003 on signed app (see `USABILITY_REGRESSION_2026_07_07.md`)
2. RC gate step 9 manual supplement (`docs/QA.md`)
3. Optional: strengthen KF-03 to assert clipboard **detail** open (AX Return on `open-detail` row is flaky; module smoke covers clipboard detail factory)

Update [`RC_BLOCKERS.md`](RC_BLOCKERS.md) when UR manual paths pass.
