# P3.2 Test Organization Report

**Date:** 2026-07-07  
**Phase:** 18 — P3.2  
**Prerequisites:** P3.1 Go (`P3_DOCS_GOVERNANCE_REPORT.md`), P2 Exit Go (`P2_EXIT_SUMMARY.md`)  
**Scope:** Documentation and test-index organization only — no full E2E, no harness rewrite, no source changes.

---

## 1. Files changed

| File | Change |
|------|--------|
| `docs/QA.md` | Added § MVP Flow Test Map |
| `P3_TEST_ORGANIZATION_REPORT.md` | This report |
| `REFACTOR_PLAN.md` | Phase 18 / P3.2 complete |

---

## 2. Test layering (current state)

| Layer | What it proves | Where | Release role |
|-------|----------------|-------|--------------|
| **Unit** | Module actors, Core dispatch, persistence primitives, redaction | `Tests/LumaModulesTests/`, `Tests/LumaCoreTests/`, `Tests/LumaInfrastructureTests/` | Required via `swift test`; filters in `docs/QA.md` |
| **Integration / harness** | Query → snapshot without full AppKit panel; partial router parity | `Tests/LumaAppTests/Flow/LauncherFlowHarness.swift`, `LauncherGoldenReplayTests.swift`, `StabilizationFlowTests.swift` | Required via `swift test`; **not** signed-runtime proof |
| **Production smoke** | Signed `Luma.app`, real `ModuleBootstrapper` + `AppHostService`, JSON artifacts | `./scripts/run_p0_smokes.sh` (`LUMA_QA_*`) | **Mandatory** P0 release gate |
| **Manual QA** | Hotkey feel, Esc chains, multi-monitor, VoiceOver, AX banners | `docs/QA.md` § Core Manual Smoke, § P0 manual supplement | Required for release tag; not fully scripted |

**Rule:** `swift test` green **and** `run_p0_smokes.sh` green **and** manual hotkey supplement — none alone is sufficient for P0 release (C-TEST-001, C-TEST-004).

---

## 3. MVP flow coverage summary

| Flow (`PRODUCT_FLOWS.md`) | Primary automated coverage | Smoke | Manual gap |
|---------------------------|---------------------------|-------|------------|
| 1 Startup | Implicit in all smokes | App launch | Fresh install / LaunchAgent |
| 2 Hotkey register | `HotkeyReregister`, policy tests | Latency in export | Cmd+Space ×20 |
| 3 Summon launcher | `HotkeyDoubleFire`, `LauncherMenuBarShowEntry` | — | Menu bar Show |
| 4 Empty home | `emptyQueryHomeGuideHasRows`, `BackHome` | — | Open Apps column |
| 5 Input query | `QueryView`, `KeystrokeReplayPerformance` | — | IME composition |
| 6 Global search | `GlobalSearchDispatch`, `LauncherGoldenReplay` | — | Tier feel |
| 7 Targeted module | `AppsModuleTests`, `Clipboard`, `Notes`, `MVPModuleDiagnostic` | Per-module smokes | — |
| 8–9 Selection / action | `LauncherActionDispatch`, smokes Return | Apps/Clipboard | External app focus |
| 10–11 Detail / back | `DetailHierarchy`, `LauncherDetailLifecycle` | Notes/Clipboard detail | Esc restore |
| 12 Hide panel | `LauncherPanelVisibilitySession`, `HideDuringSnapshotApply` | Auto-exit after smoke | Rapid toggle ×50 |
| 13 Permissions | `MVPModuleDiagnostic`, `PermissionBanner` | Clipboard AX | AX banner surfaces |
| 14 Cold start | `ModuleColdCache`, `MVPModuleDiagnostic` | — | — |
| 15 Config corrupt | `JSONConfigPersistence`, `CommandsModuleDoctor` | — | Multi-path quarantine |
| 16 Diagnostics | `DiagnosticsExport`, `LUMA_QA_EXPORT` | `diagnostics.json` | Doctor alert UI |

---

## 4. `LauncherFlowHarness` partial parity

### Aligned (P2.5)

| Area | Production | Harness |
|------|------------|---------|
| Module registration | `BuiltInModules.makeAll()` | Same |
| Global search IDs | `ModuleRegistry.globalSearchModuleIDs` | `configureGlobalSearchModuleIDs` |
| Enabled set | `config.enabledModules()` / defaults | `applyEnabledSet(defaultEnabledModuleIDs)` |
| Command registry | `ModuleRegistry.makeCommandRegistry()` | Same registry passed to `CommandRouter` |
| Query dispatch | `QueryDispatcher` + `ModuleHost` | Same stack |
| Disabled-module diagnostic | `QueryDispatcher` | `harnessDefaultOffPrefixYieldsDisabledDiagnostic` |

### Not aligned (P3 backlog — do not treat harness as signed-runtime proof)

| Gap | Impact |
|-----|--------|
| No `AppCoordinator` / `ModuleBootstrapper` async startup | Warmup timing, `modulesReady` gate, memory-pressure teardown untested |
| No `LauncherRootController` / `LauncherWindowController` | Panel show/hide, snapshot apply coalescer, detail presenter absent |
| No Carbon hotkey / `LauncherPanel` | Hotkey path entirely outside harness |
| Noop platform clients (`NoopPasteboardClient`, etc.) | Real AX/pasteboard/Workspace behavior not exercised |
| No `LauncherEnvironment.install()` | Detail registry cross-refs, production singleton wiring |
| No signed-app / TCC / Gatekeeper | Only `LUMA_QA_*` smokes cover real runtime |
| Harness `showPanel()` is a boolean flag | Does not drive AppKit visibility session |

**Verdict:** Harness validates **query routing + module fan-out logic**. It must **not** be the only evidence for hotkey, panel chrome, detail cross-fade, or production startup.

---

## 5. P0 module test ↔ smoke matrix

| Module | Unit filters | Smoke env | Artifact |
|--------|--------------|-----------|----------|
| Apps | `AppsModuleTests`, `AppsMemoryTop`, `MVPModuleDiagnostic` | `LUMA_QA_APPS=1` | `apps-smoke.json` |
| Clipboard | `Clipboard`, `MVPModuleDiagnostic` | `LUMA_QA_CLIPBOARD=1` | `clipboard-smoke.json` |
| Notes | `Notes`, `MVPModuleDiagnostic` | `LUMA_QA_NOTES=1` | `notes-smoke.json` |
| Settings | `ConfigurationStore`, `EnabledModulesMigration` | `LUMA_QA_SETTINGS=1` | `settings-smoke.json` |
| Diagnostics | `DiagnosticsExport`, `CommandsModuleDoctor` | `LUMA_QA_EXPORT=1` | `diagnostics.json` |

Runner: `./scripts/run_p0_smokes.sh` (sets `LUMA_QA_AUTO_EXIT=1` internally).

---

## 6. Build / test run

```bash
swift build   # ✅ docs-only phase; no compile changes
```

Full `swift test` not re-run for this docs-only phase (801/801 at P2 Exit).

---

## 7. P3.2 verdict

**Go** — Test map published in `docs/QA.md`; harness gaps explicit; full E2E / full parity deferred to P3 backlog.

---

## 8. Next step: Phase 19 (P3.3 or P3.4)

Recommended: **P3.4 Release hardening** (release checklist consolidation) or **P3.3 Performance budgets** (`latency-report.json` ↔ `docs/ENGINEERING.md`). Full harness ↔ `AppCoordinator` parity remains **out of scope** until explicitly planned.
