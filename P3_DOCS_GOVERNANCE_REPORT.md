# P3.1 Docs Governance Report

**Date:** 2026-07-07  
**Phase:** 17 — P3.1  
**Baseline:** P2 Exit Go (`8539007c`, `P2_EXIT_SUMMARY.md`)  
**Scope:** Documentation only — no Swift, scripts, or tests changed.

---

## 1. Files changed

| File | Change |
|------|--------|
| `docs/ENGINEERING.md` | Layer table; `LauncherContentMode` / `LauncherSessionState`; diagnostics ownership table + paths + recovery entry |
| `docs/PERMISSIONS.md` | Diagnostics ownership + both on-disk paths + menu bar recovery |
| `docs/MODULES.md` | Menu bar recovery entry; export path via `RecoveryDiagnosticsCollector` |
| `docs/QA.md` | Release checklist references `./scripts/run_p0_smokes.sh` explicitly |
| `CONTRACTS.md` | Resolved/unresolved deviation updates (per-contract + consolidated) |
| `REFACTOR_PLAN.md` | Phase 17 P3.1 complete; dependency graph |
| `P3_DOCS_GOVERNANCE_REPORT.md` | This report |

---

## 2. Stale references found

| Area | Stale statement | Source |
|------|-----------------|--------|
| Diagnostics ownership | All diagnostics attributed to `LumaInfrastructure` | `docs/ENGINEERING.md` layer table |
| Diagnostics trigger | Only `cmd export-diagnostics` | `docs/ENGINEERING.md` perf bullet |
| `crash-log.txt` path | Missing or assumed `~/Library/Logs/Luma/` | `docs/PERMISSIONS.md`, Phase 0 artifacts, `CONTRACTS.md` #5 |
| `LauncherContentMode` | "in `LauncherContentCoordinator`" (type location) | `docs/ENGINEERING.md`, C-UI-003 |
| `LauncherSessionState` | Implied production owner in some planning docs | `REFACTOR_PLAN.md` P1.2 open decision |
| Recovery entry | `cmd doctor`/`export-diagnostics` unreachable on fresh install | `CONTRACTS.md` #3, C-DIAG-001, C-DEFAULT-005 |
| Payload fields | `platform`/`modules`/etc. empty at call site | `CONTRACTS.md` #4, C-DIAG-002 |
| Release gate | Signed-app smoke implied but script not named in checklist | `docs/QA.md` Release § |
| Harness | "diverges" / `launcherFlowHarnessReplaysQuery` fails | `CONTRACTS.md` #13 (pre-P2.5) |

**Not in scope (out of allowed file list):** `MODULE_MATRIX.md` Windows row still says `defaultEnabled: true` in summary table — left for P3.2 or a matrix-only pass; `docs/MODULES.md` and manifests are authoritative.

---

## 3. What was corrected

### Diagnostics / logging
- Documented split: `LumaCore/Util` (payload/export/recording) vs `LumaInfrastructure` (`CrashLogBuffer`) vs `LumaApp` (`RecoveryDiagnosticsCollector`, `AppHostService`).
- Both paths: `~/Library/Logs/Luma/diagnostics.json`, `~/Library/Application Support/Luma/crash-log.txt`.
- Menu bar **Run Doctor…** / **Export Diagnostics…** as default-install recovery (P0.8).
- `run_p0_smokes.sh` + `LUMA_QA_EXPORT` referenced in engineering handbook.

### Launcher state
- `LauncherContentMode` type in `LauncherKeyRouter.swift`; runtime owner `LauncherContentCoordinator`.
- `LauncherSessionState` documented as test-only, not production SoT.
- `LauncherRootController` orchestration note retained.

### Defaults / modules
- No P2.1 redo; confirmed `docs/PERMISSIONS.md` / `docs/MODULES.md` already aligned post-P2.1.
- Workbench row: not a `ModuleRegistry` module (unchanged, still correct).

### QA / release
- Release checklist names `./scripts/run_p0_smokes.sh`.
- `LUMA_QA_AUTO_EXIT=1` remains runner/CI-only (from P2 Exit docs).

---

## 4. What remains unresolved

| Item | Track |
|------|-------|
| Windows `handle` → `CGWindowListCopyWindowInfo` while deferred | Code fact; not registered |
| `LauncherSessionState` delete vs promote | Product / P3 backlog |
| Parked-module AppKit scanner warns | P3 backlog |
| Full `LauncherFlowHarness` ↔ `AppCoordinator` parity | **P3.2** |
| Todo default-on product decision | User |
| `ConfigCorruptionRegistry` in-memory only | C-FAIL-006 |
| Multi-path quarantine / doctor blind spots | C-PERSIST-002 |
| Diagnostic asymmetry (non-`.queryable` silent empty) | C-FAIL-005 |
| `CrashLogBuffer` write failure without in-app alert | C-DIAG-003 partial |
| Phase 0 snapshot items (hotkey p95, historical `.ips`, scanner uncommitted edits) | Re-verify on release; not re-litigated in P3.1 |
| `MODULE_MATRIX.md` summary table drift | Optional follow-up doc pass |

---

## 5. CONTRACTS.md resolved / unresolved

### Resolved (this phase or prior, documented)

| ID / item | Resolution |
|-----------|------------|
| C-DEFAULT-004 | P2.1 — PERMISSIONS defaults + naming |
| Windows manifest metadata | P2.1 — `defaultEnabled: false` |
| C-DEFAULT-005 | P0.8/P3.1 — menu bar recovery entry |
| C-DIAG-001 reachability | P0.8/P3.1 — export/doctor via menu bar + smoke |
| C-DIAG-002 | P0.8/P2.5 — `RecoveryDiagnosticsCollector` + `LUMA_QA_EXPORT` |
| C-DIAG-004 paths | P3.1 — both paths in ENGINEERING + PERMISSIONS |
| C-UI-003 type location | P3.1 — docs match code |
| C-LAYER-001 diagnostics docs | P3.1 — ownership table |

### Unresolved (active)

| ID / item | Notes |
|-----------|-------|
| C-MODULE-006 / Windows `handle` | Deferred module |
| C-TEST-004 | Partial P2.5; full parity P3.2 |
| C-FAIL-005, C-FAIL-006, C-PERSIST-* | Corruption/diagnostic consistency |
| C-DIAG-003 | No alert without export |
| C-UI-001 | Menu bar Show bypass (documented entry, behavior unchanged) |
| LauncherSessionState | Test-only; #21 consolidated |

---

## 6. Build result

```bash
swift build
# Build complete (docs-only phase; no compile changes expected)
```

Full `swift test` not required for docs-only per phase spec.

---

## 7. P3.1 verdict

**Go** — Key engineering docs reflect post-P0/P1/P2 system state. Allowed-file scope complete.

---

## 8. Next step: P3.2 Test Organization

1. Map `swift test` filters + `run_p0_smokes.sh` to MVP flows (`PRODUCT_FLOWS.md`).
2. Label unit vs integration vs signed-app smoke in `docs/QA.md`.
3. Document remaining `LauncherFlowHarness` gaps vs `AppCoordinator` (do not implement full E2E in P3.2 unless scoped).
