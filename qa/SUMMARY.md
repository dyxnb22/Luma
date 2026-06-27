# Luma QA Summary

## Current QA Baseline

Luma is already broadly formed. QA now focuses on validating that existing launcher flows, module triggers, detail views, permissions, and cross-module actions feel connected and trustworthy.

Primary entry points:

- One-command review: `scripts/run_recorded_review.sh`
- Scripted smoke: `scripts/qa/run_full_smoke.sh`
- Environment prep: `scripts/qa/prep_smoke_env.sh`
- Recorded review brief: `docs/RECORDED_QA_BRIEF.md`
- Manual checklist: `docs/MANUAL_QA_CHECKLIST.md`
- Findings template: `qa/RECORDED_REVIEW_TEMPLATE.md`

## Findings by Round

| Round | P0 | P1 | P2 | P3 |
|-------|----|----|----|-----|
| 1 | 0 | 0 | 1 | 4 |
| 2 | 0 | 0 | 0 | 0 |
| Final (prior) | 0 | 0 | 0 | 3 |

## Final Regression State

- Round 2 recorded/smoke baseline: **pass** (no new P2+).
- Fixes shipped this session: empty search placeholder (`No matching results.`), Browser Tabs exact-duplicate record dedup without collapsing distinct same-URL tabs.
- Open items: Todo Reminders is still **host permission/setup** (P3, actionable UI).
- The canonical smoke entry point is `scripts/qa/run_full_smoke.sh`.
- Findings log: `qa/RECORDED_REVIEW_TEMPLATE.md` (Round 3 session).

## Performance

- **Fast path:** `KeystrokeReplayPerformanceTests` — global search replay, p95 < 30 ms (Browser Tabs default-off)
- **Slow modules:** `SlowModuleQueryPerformanceTests` — `tab` p95 < 950 ms, warm cache p95 < 50 ms; `kill preview` p95 < 200 ms
- **Doctor:** `run_doctor_perf.sh` includes `tab github` and `kill preview`
- Latency HUD enabled in QA via `latencyHUDEnabled` default

## Current Priorities

- Preserve the fast path while polishing detail surfaces.
- Validate default-off modules when enabled in realistic sessions.
- Catch doc/code mismatches early.
- Keep issue logging reproducible and timestamped for recorded reviews.
