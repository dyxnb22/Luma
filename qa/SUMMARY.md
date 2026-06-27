# Luma QA Summary

## Implementation Overview

Four modules (Quicklinks, Menu Items, Kill Process, Browser Tabs) plus QA harness and launcher fixes.

| Area | Key files |
|------|-----------|
| QA driver | `scripts/qa/drive.sh`, `paste_query.swift`, `prep_smoke_env.sh` |
| Final smoke | `qa/final/run_smoke.sh`, `qa/final/run_doctor_perf.sh` |
| Ranker | `Sources/LumaCore/Ranking/Ranker.swift` |
| Browser tabs | `SafariAdapter.swift`, `ChromiumAdapter.swift`, `BrowserTabsService.swift` |
| Projects QA seed | `scripts/qa/prep_smoke_env.sh` → `projects.json` |

## Findings by Round

| Round | P0 | P1 | P2 | P3 |
|-------|----|----|----|-----|
| 1 | 0 | 3 | 2 | 0 |
| 2 | 0 | 0 | 1 | 1 |
| Final | 0 | 0 | 0 | 3 |

## Final session fixes

1. **Browser tab AppleScript** — `tab` inside `tell application "Safari"` is the tab class, not ASCII 9; use `ASCII character 9`.
2. **Ranker home views** — empty command payload must not fuzzy-filter on trigger (`proj`, `tab`, `clip`, …).
3. **Browser tab cache** — `searchableTabs()` blocks until refresh when cache empty/stale.
4. **Smoke prep** — seeds projects root, enables `luma.browser-tabs`, opens Safari GitHub tab, restarts app.

## Performance

- `KeystrokeReplayPerformanceTests.appSearchThousandKeystrokeReplayStaysUnderBudget`: p95 < 30 ms
- Latency HUD enabled in QA via `latencyHUDEnabled` default

## Screenshots

- `qa/final/screenshots/` — full trigger smoke (2026-06-27)
- `qa/final/screenshots/doctor-latency-hud.png` — perf HUD after isolated queries

## Stop Criteria Status

- [x] Four modules implemented and registered
- [x] `swift test` green (368 tests)
- [x] App builds (`./scripts/build_app.sh`)
- [x] Full screenshot smoke via `qa/final/run_smoke.sh`
- [x] Final round P0/P1/P2 = 0
- [x] Keystroke replay p95 ≤ 30 ms (unit test)
