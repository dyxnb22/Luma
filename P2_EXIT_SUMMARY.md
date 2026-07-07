# P2 Exit Summary

**Date:** 2026-07-07  
**P0 baseline commit:** `889ebd35` — *Add QA smoke hooks and config corruption tracking* (`P0_EXIT_SUMMARY.md`)  
**P1 exit commit:** `bc966c29` — *Refine launcher show-entry and session governance* (`P1_EXIT_SUMMARY.md`)  
**P2 exit commit:** `8539007c` — *Add production smoke completion hooks and main-actor wrappers*  
**Branch:** `main` (clean working tree at exit gate)  
**P2 Exit verdict:** **Go** ✅

---

## 1. Baseline commits

| Milestone | Commit | Role |
|-----------|--------|------|
| P0 code + smoke hooks | `889ebd35` | Frozen MVP functional baseline |
| P1 exit | `bc966c29` | Launcher governance complete |
| P2 planning | `1a384126` | `P2_ROADMAP.md`, scope audit, decision matrix |
| **P2 execution + exit** | **`8539007c`** | P2.1–P2.5 delivered |

Companion: `PHASE15_P2_EXECUTION_REPORT.md` (slice detail + P2.5 runner validation).

---

## 2. P2.1–P2.5 delivered

| Slice | Status | Key deliverables |
|-------|--------|------------------|
| **P2.1** Documentation / Manifest Hygiene | ✅ | `docs/PERMISSIONS.md` defaults; `docs/MODULES.md` registration table; Windows `defaultEnabled: false` (metadata); `LAUNCHER_STATE_AUDIT.md` test-only stamp; C-DEFAULT-004 resolved in `CONTRACTS.md` |
| **P2.2** Module Diagnostic Consistency | ✅ | Failure taxonomy in `docs/MODULES.md`; Notes bare `n` onboarding when root unset; `MVPModuleDiagnosticTests` (Apps/Clipboard/Notes) |
| **P2.3** Lifecycle Contract Tests | ✅ | `ModuleHandleContractTests` extended; `scripts/scan_handle_memory_only.sh`; lifecycle exceptions appendix |
| **P2.4** Core P1 AppKit Cleanup | ✅ | Snippets/Quicklinks/Translate/Todo detail views — 0 scanner warns per file |
| **P2.5** QA Harness / Smoke Runner | ✅ | `scripts/run_p0_smokes.sh` (validated); `ProductionSmokeSupport` + `LUMA_QA_AUTO_EXIT=1`; partial `LauncherFlowHarness` production align |

---

## 3. Changed files (P2 execution: `6709fad6..8539007c`)

| Area | Files |
|------|-------|
| Docs | `docs/PERMISSIONS.md`, `docs/MODULES.md`, `docs/QA.md`, `CONTRACTS.md`, `LAUNCHER_STATE_AUDIT.md`, `PHASE15_P2_EXECUTION_REPORT.md` |
| Modules | `WindowsModule.swift` (manifest only), `NotesModule.swift` (onboarding) |
| Launcher UI | `SnippetsDetailView.swift`, `QuicklinksDetailView.swift`, `TranslateDetailView.swift`, `TodoDetailView.swift` |
| Smoke / infra | `ProductionSmokeSupport.swift`, `*ProductionSmoke.swift`, `AppCoordinator.swift` |
| Tests | `MVPModuleDiagnosticTests.swift`, `ModuleHandleContractTests.swift`, `LauncherFlowHarness.swift`, `StabilizationFlowTests.swift` |
| Scripts | `scripts/run_p0_smokes.sh`, `scripts/scan_handle_memory_only.sh` |

**24 files, +824 / −86 lines** (single Phase 15 commit on top of planning docs).

---

## 4. Behavioral changes (intentional)

| Change | Scope | MVP impact |
|--------|-------|------------|
| **Windows manifest** | `defaultEnabled: false` only; still in `BuiltInModules.makeDeferred()` | None — not registered |
| **Notes bare `n` without root** | Returns onboarding row "Choose a Notes root folder" | P0 Notes path clearer (C-FAIL-003) |
| **Core P1 AppKit `@objc`** | `nonisolated` + `Task { @MainActor }` on target/actions | No feature change; crash-class hardening |
| **`LUMA_QA_AUTO_EXIT=1`** | Set only by `run_p0_smokes.sh` / smoke runner | Normal launch unchanged; app terminates after JSON write in QA only |
| **Harness default-off prefix** | `mb` on fresh install → disabled diagnostic row | Matches production `QueryDispatcher` behavior |

**Not changed:** ModuleHost, QueryDispatcher, runtime `defaultEnabled` on registered modules, Todo/Snippets/Quicklinks/Translate default state, parked module registration.

---

## 5. Tests and scripts run (Phase 16 exit gate)

| Check | Result (2026-07-07) |
|-------|---------------------|
| `swift build` | ✅ |
| `swift test` | ✅ **801/801** |
| `bash scripts/scan_handle_memory_only.sh` | ✅ |
| `bash scripts/scan_appkit_executor_risk.sh` | ✅ blocking checks pass; parked/non-P2 files may still warn |
| `./scripts/build_app.sh --no-restart` | ✅ |
| `./scripts/run_p0_smokes.sh` | ✅ exit 0 (~18s) |

---

## 6. Signed-app smoke runner

```bash
./scripts/build_app.sh --no-restart
./scripts/run_p0_smokes.sh
```

| Env | Artifact | Phase 16 |
|-----|----------|----------|
| `LUMA_QA_APPS` | `apps-smoke.json` | OK |
| `LUMA_QA_CLIPBOARD` | `clipboard-smoke.json` | OK |
| `LUMA_QA_NOTES` | `notes-smoke.json` | OK |
| `LUMA_QA_SETTINGS` | `settings-smoke.json` | OK |
| `LUMA_QA_EXPORT` | `diagnostics.json` | OK |

App exits after each artifact (`LUMA_QA_AUTO_EXIT=1` via runner). **Not** full UI automation — artifact presence + production wiring only.

---

## 7. `.ips` / process hygiene

| Metric | Phase 16 gate |
|--------|----------------|
| New `Luma` `.ips` (30 min window) | **0** |
| `pgrep -x Luma` after runner | **empty** |

---

## 8. MVP scope

**Unchanged.** P0 core remains Apps / Clipboard / Notes + Settings / Diagnostics recovery. Core P1 candidates (Snippets, Quicklinks, Translate, Todo) stay default-on per code; no new P0 gate requirements added.

---

## 9. Parked / deferred modules

**Still not enabled or registered:**

- Windows — deferred (`makeDeferred()` only)
- Media, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Wordbook, Projects, Commands (default-off) — unchanged
- Windows `handle()` CGWindow violation — **not** fixed (by design)

---

## 10. Remaining backlog → P3 / product

| Item | Track |
|------|-------|
| `docs/ENGINEERING.md` stale locations, diagnostics ownership, `crash-log.txt` path | P3.1 |
| Full `CONTRACTS.md` deviation sweep | P3.1 |
| Test organization by MVP flow (`docs/QA.md` index) | P3.2 |
| Full `LauncherFlowHarness` ↔ `AppCoordinator` parity | P3.2 |
| Parked-module AppKit warn cleanup | P3 backlog |
| `LauncherSessionState` delete vs promote | P3 / post-P2 |
| **Todo** default-on vs deferred | **User decision** |
| Release checklist hardening (mandatory `run_p0_smokes.sh`) | P3.4 |
| Performance budget / `latency-report.json` align | P3.3 |

---

## 11. P2 exit criteria checklist

| Criterion (`REFACTOR_PLAN.md` §10) | Met |
|-------------------------------------|-----|
| P0 modules lifecycle contract tested | ✅ P2.3 |
| Diagnostic behavior documented for MVP set | ✅ P2.2 taxonomy + tests |
| PERMISSIONS / MODULES / manifests agree | ✅ P2.1 |
| Parked modules re-entry criteria intact | ✅ no registration changes |
| Terminable smoke script | ✅ P2.5 validated |

---

## 12. P2 Exit verdict

**Go** — All Phase 16 regression gates green. P2 module governance complete; proceed to **P3 release hardening** (docs/tests/checklist only).

---

## 13. P3 recommended next steps (planning only)

1. **P3.1 Docs governance** — `docs/ENGINEERING.md`, diagnostics paths, remove stale duplicates; close remaining doc-only `CONTRACTS.md` deviations.
2. **P3.2 Test organization** — Map `swift test` + `run_p0_smokes.sh` to MVP flows; label harness vs production gaps.
3. **P3.3 Performance budgets** — Align `latency-report.json` with `docs/ENGINEERING.md`.
4. **P3.4 Release hardening** — Mandatory `./scripts/run_p0_smokes.sh` in release checklist; crash/log bundle instructions.
5. **Product** — Todo default-on decision; Core P1 default-on policy confirmation.
6. **Backlog** — Parked AppKit warns; full harness parity; `LauncherSessionState` fate.

*No P3 implementation in Phase 16.*
