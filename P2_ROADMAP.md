# P2 Roadmap (Phase 14.3)

**Date:** 2026-07-07  
**Baseline:** `bc966c29` (P1 exit)  
**Execution commit:** `8539007c` (Phase 15)  
**Exit gate:** **Go** — `P2_EXIT_SUMMARY.md` (Phase 16)  
**Prerequisites:** P0 Exit **Go**, P1 Exit **Go** (`P1_EXIT_SUMMARY.md`)  
**Governance principle:** Small slices; doc-first where possible; **no big bang**.

---

## Overview

```
P2.1 Documentation / Manifest Hygiene     ✅ complete
    ↓
P2.2 Module Diagnostic Consistency        ✅ complete
    ↓
P2.3 Module Lifecycle Contract Tests      ✅ complete
    ↓
P2.4 Non-MVP AppKit Warn Cleanup          ✅ complete (Core P1 only)
    ↓
P2.5 QA Harness / Smoke Runner            ✅ complete (partial harness parity)
```

**Global stop condition:** P0 gate failure → revert active slice, P0 triage. *(No active P2 slices — use for P3/post-P2 changes.)*

---

## Slice status summary

| Slice | Status | Validating tests / scripts |
|-------|--------|---------------------------|
| **P2.1** | ✅ Complete | `swift build`; doc cross-check vs manifests |
| **P2.2** | ✅ Complete | `MVPModuleDiagnosticTests` (Apps warming, Notes onboarding, Clipboard permission) |
| **P2.3** | ✅ Complete | `ModuleHandleContractTests`; `scripts/scan_handle_memory_only.sh` |
| **P2.4** | ✅ Complete | `scripts/scan_appkit_executor_risk.sh` (0 warns on Snippets/Quicklinks/Translate/Todo detail views) |
| **P2.5** | ✅ Complete | `./scripts/run_p0_smokes.sh`; `StabilizationFlowTests` / `LauncherFlowHarness` partial align |

---

## P2.1 — Documentation / Manifest Hygiene — ✅ Complete

### Delivered (`8539007c`)
- `docs/PERMISSIONS.md` Default column aligned to manifests / D-012
- `WindowsModule` manifest `defaultEnabled: false` (metadata only; still deferred)
- `docs/MODULES.md` registration status table
- `LAUNCHER_STATE_AUDIT.md` — `LauncherSessionState` test-only stamp
- `CONTRACTS.md` — C-DEFAULT-004 resolved

### Acceptance
- [x] PERMISSIONS Default matches `ModuleWarmupDefaults` + manifests
- [x] Windows manifest `defaultEnabled: false`; still deferred
- [x] MODULE_MATRIX / MODULES agree on parked list
- [x] Session state: documented test-only for P2

---

## P2.2 — Module Diagnostic Consistency — ✅ Complete

### Delivered (`8539007c`)
- Failure taxonomy in `docs/MODULES.md`
- Notes bare `n` without root → onboarding row
- `Tests/LumaModulesTests/MVPModuleDiagnosticTests.swift`

### Validating tests
```bash
swift test --filter MVPModuleDiagnostic
```

### Acceptance
- [x] Taxonomy table exists and MVP modules reference it
- [x] Notes onboarding row when root unset (C-FAIL-003)
- [x] Clipboard permission row when AX denied (C-FAIL-001)
- [x] Apps `app top` warming row documented and tested

### Deferred / partial
- Core P1 modules (Quicklinks, Snippets, Translate, Todo): taxonomy documented; no new per-module diagnostic tests beyond MVP set in this slice

---

## P2.3 — Module Lifecycle Contract Tests — ✅ Complete

### Delivered (`8539007c`)
- `ModuleHandleContractTests` extended (P0 handle memory-only + teardown source checks)
- `scripts/scan_handle_memory_only.sh`
- Lifecycle exceptions appendix in `docs/MODULES.md`

### Validating tests
```bash
swift test --filter ModuleHandleContract
bash scripts/scan_handle_memory_only.sh
```

### Acceptance
- [x] Each P0 module: handle test passes (no await/network/disk in handle path)
- [x] Teardown cancels refresh tasks (Apps, Clipboard — verified via existing tests)
- [x] Exceptions table lists deferred modules explicitly

---

## P2.4 — Non-MVP AppKit Warn Cleanup — ✅ Complete (Core P1 scope)

### Delivered (`8539007c`)
- `SnippetsDetailView`, `QuicklinksDetailView`, `TranslateDetailView`, `TodoDetailView` — `@objc nonisolated` + `Task { @MainActor }` hop
- **0 scanner warns** per touched file

### Validating tests
```bash
bash scripts/scan_appkit_executor_risk.sh
swift build
```

### Acceptance
- [x] Touched files: zero scanner warns each
- [x] `swift build` green

### Known gap (P3 backlog)
- Parked modules (Wordbook, Media, Secrets, Projects, `CurrentProjectDetailView`, etc.) still report warns — **out of P2 scope**

---

## P2.5 — QA Harness / Smoke Runner — ✅ Complete

### Delivered (`8539007c`)
- `scripts/run_p0_smokes.sh` — terminable runner; sets `LUMA_QA_AUTO_EXIT=1` internally
- `ProductionSmokeSupport` + all `*ProductionSmoke.swift` call `finish(artifact:)`
- `LauncherFlowHarness` — production `CommandRegistry`, `globalSearchModuleIDs`, `applyEnabledSet`
- `StabilizationFlowTests` — `harnessDefaultOffPrefixYieldsDisabledDiagnostic`
- `docs/QA.md` references `./scripts/run_p0_smokes.sh`

### Validating tests
```bash
./scripts/build_app.sh --no-restart
./scripts/run_p0_smokes.sh          # exit 0; ~18s; all 5 JSON artifacts
swift test --filter StabilizationFlow
```

### Acceptance
- [x] Single script exits 0/1 with JSON artifacts
- [x] Harness partial production parity (router + enabled set)
- [x] `docs/QA.md` references script

### Known gaps (P3)
| Gap | Notes |
|-----|-------|
| **Artifact polling, not full UI automation** | `run_p0_smokes.sh` validates JSON presence + production wiring; does not drive screenshots or full panel flows |
| **`LUMA_QA_EXPORT`** | Validated via `diagnostics.json` fields; no EXPORT UI automation |
| **Full `LauncherFlowHarness` ↔ `AppCoordinator` parity** | Partial align only — full E2E remains P3.2 |
| **`LUMA_QA_AUTO_EXIT=1`** | Set by runner / CI only; normal launch unchanged |

### Phase 16 exit validation
- `./scripts/run_p0_smokes.sh` → exit 0
- Artifacts: `apps-smoke.json`, `clipboard-smoke.json`, `notes-smoke.json`, `settings-smoke.json`, `diagnostics.json`
- `pgrep -x Luma` empty after run; no new `.ips` in 30 min window

---

## P2 exit criteria — ✅ Met

| Gate | Result |
|------|--------|
| Docs | C-DEFAULT-004 closed; Windows manifest fixed; parked manifest clear |
| Modules | P0 diagnostic + lifecycle contracts tested |
| Launcher | No P1 regression; session state decision recorded |
| QA | Terminable smoke script validated (`run_p0_smokes.sh`) |
| P0 | Full gate green (801/801 `swift test`, scanners, smokes) |

**Verdict:** **Go** — see `P2_EXIT_SUMMARY.md`.

---

## Post-P2 backlog (P3 — not P2)

| Item | Track |
|------|-------|
| Full docs governance (`docs/ENGINEERING.md`, diagnostics paths) | P3.1 |
| Test organization by MVP flow | P3.2 |
| Full `LauncherFlowHarness` ↔ production parity | P3.2 |
| Release checklist + mandatory smoke gate | P3.4 |
| `LauncherSessionState` delete vs promote | P3 / product |
| Todo default-on decision | **User** |
| Parked-module AppKit warns | P3 backlog |
| Clipboard history scale / Notes polish | Product |

---

*Phase 14.3 planning · Phase 15 execution · Phase 16 exit Go.*
