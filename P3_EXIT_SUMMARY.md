# P3 Exit Summary

**Date:** 2026-07-07  
**P0 baseline commit:** `889ebd35` — *Add QA smoke hooks and config corruption tracking* (`P0_EXIT_SUMMARY.md`)  
**P1 exit commit:** `bc966c29` — *Refine launcher show-entry and session governance* (`P1_EXIT_SUMMARY.md`)  
**P2 exit commit:** `8539007c` — *Add production smoke completion hooks and main-actor wrappers* (`P2_EXIT_SUMMARY.md`)  
**P3 gate commit (HEAD at exit):** `4b34fe09` — *Update contracts for resolved diagnostics and defaults*  
**Branch:** `main` (uncommitted P3.3–P3.4 docs/scripts on top of HEAD)  
**P3 Exit verdict:** **Go** ✅  
**Release Candidate verdict:** **No-Go** (pending human manual supplement — RC gate step 9)

**Phase 21.1 (2026-07-07):** Automated gate now includes mandatory `LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh` after smoke/`.ips` checks (`run_release_gate.sh` step 9/9). Fresh `LUMA_QA=1` session remains recommended, not required for the automated pass if an on-disk report exists within RC ceilings.

---

## 1. Baseline commits

| Milestone | Commit | Role |
|-----------|--------|------|
| P0 code + smoke hooks | `889ebd35` | Frozen MVP functional baseline |
| P1 exit | `bc966c29` | Launcher governance complete |
| P2 exit | `8539007c` | Module governance + `run_p0_smokes.sh` |
| P3.1 docs governance | Phase 17 (`P3_DOCS_GOVERNANCE_REPORT.md`) | ENGINEERING/PERMISSIONS/MODULES/CONTRACTS alignment |
| P3.2 test organization | Phase 18 (`P3_TEST_ORGANIZATION_REPORT.md`) | MVP Flow Test Map in `docs/QA.md` |
| **P3.3–P3.4 + exit gate** | **`4b34fe09` + working tree** | Performance budgets, RC gate script, exit validation |

Companion reports: `P3_DOCS_GOVERNANCE_REPORT.md`, `P3_TEST_ORGANIZATION_REPORT.md`, `P3_PERFORMANCE_BUDGETS_REPORT.md`, `P3_RELEASE_HARDENING_REPORT.md`.

---

## 2. P3.1–P3.4 delivered

| Slice | Status | Key deliverables |
|-------|--------|------------------|
| **P3.1** Documentation / Manifest Hygiene | ✅ | `docs/ENGINEERING.md` diagnostics ownership + paths; `CONTRACTS.md` deviation sweep; `P3_DOCS_GOVERNANCE_REPORT.md` |
| **P3.2** Test Organization | ✅ | `docs/QA.md` § MVP Flow Test Map; harness gaps labeled; `P3_TEST_ORGANIZATION_REPORT.md` |
| **P3.3** Performance Budgets | ✅ | Release-gating vs aspirational split in ENGINEERING; `docs/QA.md` § Performance Gate; `LUMA_RELEASE_GATE=1` on `export_latency_report.sh`; `P3_PERFORMANCE_BUDGETS_REPORT.md` |
| **P3.4** Release Hardening | ✅ | `docs/QA.md` § Release Candidate Gate (9-step); `./scripts/run_release_gate.sh`; `P3_RELEASE_HARDENING_REPORT.md` |

---

## 3. Changed files (Phase 17–21, docs/scripts scope)

| Area | Files |
|------|-------|
| Docs | `docs/ENGINEERING.md`, `docs/QA.md`, `docs/PERMISSIONS.md`, `docs/MODULES.md`, `CONTRACTS.md` |
| Reports | `P3_DOCS_GOVERNANCE_REPORT.md`, `P3_TEST_ORGANIZATION_REPORT.md`, `P3_PERFORMANCE_BUDGETS_REPORT.md`, `P3_RELEASE_HARDENING_REPORT.md`, `P3_EXIT_SUMMARY.md` |
| Scripts | `scripts/run_release_gate.sh` (new), `scripts/qa/export_latency_report.sh`, `scripts/run_p0_smokes.sh` |
| Plan | `REFACTOR_PLAN.md` |

**No `Sources/**` changes in P3 exit scope.** MVP module registration and `defaultEnabled` unchanged.

---

## 4. Final gate results (2026-07-07)

| Check | Result |
|-------|--------|
| `git status` (informational) | Dirty tree — P3.3–P3.4 docs/scripts uncommitted |
| `swift build` | ✅ |
| `swift test` | ✅ **801/801** (6 suites, ~5.2 s) |
| `bash scripts/scan_handle_memory_only.sh` | ✅ |
| `bash scripts/scan_appkit_executor_risk.sh` | ✅ blocking checks pass; parked-module WARNs expected |
| `./scripts/build_app.sh --no-restart` | ✅ signed `build/Luma.app` |
| `./scripts/run_p0_smokes.sh` | ✅ exit 0 (~26 s) |
| Smoke artifacts | ✅ `apps-`, `clipboard-`, `notes-`, `settings-smoke.json`, `diagnostics.json` |
| `.ips` before / after | **0 / 0** (no new crash reports) |
| `./scripts/run_release_gate.sh` | ✅ exit 0 (Phase 21.1: includes mandatory latency check) |
| `LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh` | ✅ hotkey **28.1 ms**, keystroke **20.0 ms** (within RC ceilings) |

### Performance gate (Phase 19 + 21.1)

| Metric | Source | Value | RC ceiling | Pass |
|--------|--------|-------|------------|------|
| Hotkey p95 | `latency-report.json` (prior `LUMA_QA=1` session, 2026-07-06) | **28.1 ms** | ≤ 1000 ms | ✅ |
| Keystroke p95 | same | **20.0 ms** | ≤ 60 ms | ✅ |
| Combined p95 | same | 27.8 ms | informational | — |
| `diagnostics.json` `latencyP95Milliseconds` | smoke export | **0** (smoke path; no hotkey samples) | reference only | — |

```bash
LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh
# Mode: release-gate (hotkey ≤ 1000 ms, keystroke ≤ 60 ms) — PASS
```

**Automated gate:** `run_release_gate.sh` fails if `latency-report.json` is missing or over RC budgets. **Recommended** before RC tag: fresh `LUMA_QA=1` session to refresh samples (`docs/QA.md` § Performance Gate). Smokes alone do not update `latency-report.json`.

### Manual supplement (RC gate step 9)

| Item | Status |
|------|--------|
| Cmd+Space show/hide ×20 (or menu bar Show) | **Not recorded** — requires human operator |
| Esc / hide / reshow → search field editable | **Not recorded** |
| Menu bar: Show, Run Doctor…, Export Diagnostics… | **Not recorded** |
| Confirm no new `.ips` during manual pass | Automated count 0/0 only |

---

## 5. Signed-app smoke runner

```bash
./scripts/build_app.sh --no-restart
./scripts/run_p0_smokes.sh
# Or full automated RC steps 2–8 + mandatory latency check:
./scripts/run_release_gate.sh
```

| Env | Artifact | P3 exit gate |
|-----|----------|--------------|
| `LUMA_QA_APPS` | `apps-smoke.json` | OK |
| `LUMA_QA_CLIPBOARD` | `clipboard-smoke.json` | OK |
| `LUMA_QA_NOTES` | `notes-smoke.json` | OK |
| `LUMA_QA_SETTINGS` | `settings-smoke.json` | OK |
| `LUMA_QA_EXPORT` | `diagnostics.json` | OK |

---

## 6. Explicit defer backlog (not P3 exit blockers)

| Item | Track |
|------|-------|
| Full `LauncherFlowHarness` ↔ `AppCoordinator` parity | P3 backlog — labeled in `docs/QA.md`; `CONTRACTS.md` #13 unresolved |
| Full UI E2E automation | Out of scope — manual supplement remains mandatory for RC |
| Parked-module AppKit `@objc` WARNs | P3+ backlog — scanner blocking checks pass |
| **Todo** default-on vs deferred | **User / product decision** (`MVP_SCOPE.md`) |
| `LauncherSessionState` delete vs promote | Post-P3 |
| Windows deferred `handle` / `CGWindowListCopyWindowInfo` | Parked — not registered |
| `MODULE_MATRIX.md` Windows summary row vs manifest | Doc-only; `docs/MODULES.md` authoritative |
| Fresh `LUMA_QA=1` latency session per release tag | Recommended (automated gate uses existing report) |

---

## 7. P3 exit criteria checklist (`REFACTOR_PLAN.md` §10)

| Criterion | Met |
|-----------|-----|
| Doc/code mismatches from `CONTRACTS.md` deviations resolved or explicitly labeled | ✅ P3.1 sweep + harness #13 labeled |
| Tests/QA organized around MVP flows; harness divergence explicit | ✅ P3.2 MVP Flow Test Map |
| Performance budgets align with ENGINEERING release-gating ceilings | ✅ P3.3 — 1000/60 ms RC; aspirational 50/80/30 documented |
| Signed-app smoke referenced as release gate; catches P0-class failures | ✅ P3.4 — `run_release_gate.sh` + `run_p0_smokes.sh` mandatory |

---

## 8. P3 Exit verdict

**Go** — P3.1–P3.4 deliverables complete. Automated Release Candidate Gate (steps 2–8 + mandatory latency check) green on 2026-07-07. Defer backlog documented; no MVP scope expansion.

---

## 9. Release Candidate verdict

**No-Go** — Automated gate (including mandatory `latency-report.json` check) and P3 documentation criteria are satisfied, but **RC gate step 9 (manual supplement) was not executed and recorded in this exit run**. Before tagging a release candidate:

1. Complete manual supplement in `docs/QA.md` § Release Candidate Gate step 9.
2. **Recommended:** refresh latency with `LUMA_QA=1 build/Luma.app/Contents/MacOS/Luma` → hotkey/keystroke exercise → re-run `./scripts/run_release_gate.sh`.
3. Re-run `./scripts/run_release_gate.sh` on a clean or release branch if desired.

**Product blockers (unchanged):** Todo default-on open decision; Core P1 default-on policy confirmation — not engineering gate failures.

---

## 10. Recommended post-P3

1. Commit P3.3–P3.4 docs/scripts + `P3_EXIT_SUMMARY.md`.
2. Human RC step 9 → flip RC verdict to **Go** if manual checks pass.
3. Product: Todo default-on decision; consider parked AppKit warn cleanup as incremental PRs.
4. Engineering backlog: harness parity (optional); `LauncherSessionState` fate.
