# P3.4 Release Hardening Report

**Date:** 2026-07-07  
**Phase:** 20 — P3.4  
**Scope:** Release checklist consolidation + `run_release_gate.sh` orchestrator.

---

## 1. Files changed

| File | Change |
|------|--------|
| `docs/QA.md` | § Release Candidate Gate (9-step); Release checklist points to RC gate |
| `scripts/run_release_gate.sh` | **New** — automates steps 2–8 |
| `scripts/run_p0_smokes.sh` | Clearer failure output (artifact path, pgrep hint) |
| `P3_RELEASE_HARDENING_REPORT.md` | This report |

---

## 2. RC gate flow

| Step | Action | Automated |
|------|--------|-----------|
| 1 | `git status` / `git log -1` | Informational in script |
| 2 | `swift build` | Yes |
| 3 | `swift test` | Yes |
| 4 | `scan_handle_memory_only` + `scan_appkit_executor_risk` | Yes |
| 5 | `./scripts/build_app.sh --no-restart` | Yes |
| 6 | `./scripts/run_p0_smokes.sh` | Yes (mandatory signed-app gate) |
| 7 | Verify 5 JSON artifacts | Yes |
| 8 | `.ips` before/after | Yes |
| 9 | Manual supplement + optional latency | **No** (operator) |

---

## 3. Manual supplement (step 9)

- Cmd+Space ×20 or menu bar Show
- Esc / hide / reshow input
- Menu bar Doctor / Export Diagnostics
- No new `.ips`
- Recommended: `LUMA_QA=1` + `LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh`

---

## 4. Not in scope

- Full UI automation framework
- Parked / Core P1 / Todo as release blockers
- `LUMA_QA=1` inside smoke runner (preserves smoke behavior)

---

## 5. Relationship to P0 gate

`run_release_gate.sh` **supersedes** ad-hoc P0 gate runs for RC; it calls `run_p0_smokes.sh` internally. § P0 MVP Smoke Gate remains reference for per-smoke manual fallback.

---

## 6. P3.4 verdict

**Go** pending Phase 21 full gate execution — see `P3_EXIT_SUMMARY.md`.
