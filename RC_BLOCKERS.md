# Release Candidate Blockers

**Last updated:** 2026-07-07 (Phase 22 exit)

---

## Active blockers

### Phase 22 — Launcher keyboard E2E usability (2026-07-07)

**Verdict:** RC **No-Go** — automated Phase 22 gate green; manual UR re-test + RC step 9 remain.

| Blocker | Detail |
|---------|--------|
| Manual usability re-test | [`USABILITY_REGRESSION_2026_07_07.md`](USABILITY_REGRESSION_2026_07_07.md) UR-001–UR-003 need human confirmation after fixes |
| Automation | `./scripts/qa/run_keyboard_flows.sh` — **PASS** (2026-07-07, failures 0, ips delta 0) |
| Invariants | `launcher-state-violations.json` written under `LUMA_QA=1`; checker I1–I6 in [`LauncherStateSnapshot.swift`](Sources/LumaCore/Launcher/LauncherStateSnapshot.swift) |

**Lift criteria:**

1. Manual UR-001–UR-003 pass on signed `build/Luma.app`
2. `./scripts/qa/run_keyboard_flows.sh` exit 0 (automated ✅)
3. RC gate step 9 manual supplement (`docs/QA.md` § Release Candidate Gate)

See [`PHASE_22_EXIT_SUMMARY.md`](PHASE_22_EXIT_SUMMARY.md).

---

### RC gate step 9 — Manual supplement (unchanged from P3 exit)

**Verdict:** RC **No-Go** until human operator records:

- Cmd+Space show/hide ×20 (or menu bar Show)
- Esc / hide / reshow → search field editable
- Menu bar: Show, Run Doctor…, Export Diagnostics…
- No new `.ips` during manual pass

See [`P3_EXIT_SUMMARY.md`](P3_EXIT_SUMMARY.md) §9.

---

### Product (not engineering gate)

- Todo default-on vs deferred — user/product decision (`MVP_SCOPE.md`)

---

## Resolved blockers

| Phase | Blocker | Resolved |
|-------|---------|----------|
| P3 | Automated release gate (build, test, smokes, latency) | 2026-07-07 — `run_release_gate.sh` green |
