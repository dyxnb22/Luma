# Final QA Round — Findings

Date: 2026-06-27

## Resolved (this session)

| ID | Severity | Issue | Root cause | Fix |
|----|----------|-------|------------|-----|
| F-F-01 | P1 | `proj luma` empty | No `projects.json` roots | `scripts/qa/prep_smoke_env.sh` seeds `~/Luma` root |
| F-F-02 | P1 | `tab github` empty | AppleScript `tab` inside `tell Safari` resolves to tab class, not delimiter | `ASCII character 9` in Safari/Chromium adapters |
| F-F-03 | P1 | `proj` / `tab` home rows empty | Ranker filtered on trigger token when payload empty | Empty command payload → no fuzzy filter |
| F-F-04 | P2 | Stale browser tab cache on first query | `cachedTabs()` fired async refresh but returned immediately | `searchableTabs()` awaits refresh when stale/empty |
| F-F-05 | P2 | Doctor perf query pollution | `run_doctor_perf.sh` typed into same field | Close/open between each query |

## Prior rounds (already fixed)

- Ranker payload vs trigger token (`gh swift package`, `kill preview`)
- Safari symlink in AppIndex
- Menu bar cache clear + AX walk budget
- QA session restore / paste_query keycodes

## Final status

- P0/P1/P2: **0 open**
- `swift test`: **368 passed**
- Full smoke (`qa/final/run_smoke.sh`): **pass** (screenshots reviewed)
- Keystroke replay perf test: p95 < 30 ms (unit test)

## Known P3

- Browser Tabs module remains **default-off**; QA enables via `enabledModules` in prep script.
- `tab` smoke may show duplicate GitHub rows when prep opens multiple Safari tabs.
- Automation permission required per browser (Safari granted on this machine).
