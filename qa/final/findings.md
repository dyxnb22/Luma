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
| F-F-06 | P1 | Module diagnostics invisible in launcher | `QueryDispatcher` dropped `ModuleResult.diagnostic`; no UI row | `ModuleDiagnosticResults` + merge in dispatcher |
| F-F-07 | P1 | Module toggle did not warmup/teardown | `applyEnabledSet` only swapped enabled set | Diff enabled set; call `warmup`/`teardown` on change |
| F-F-08 | P2 | Perf gate missed slow module paths | Keystroke replay only covered global search | `SlowModuleQueryPerformanceTests` + doctor `tab`/`kill` queries |
| F-F-09 | P2 | Duplicate smoke entry points | Multiple smoke wrappers diverged | Canonical `scripts/qa/run_full_smoke.sh` |
| F-F-10 | P2 | Safari tests in default gate | `/Applications/Safari.app` required in CI | `@Tag integration` + `scripts/test_unit.sh` |

## Prior rounds (already fixed)

- Ranker payload vs trigger token (`gh swift package`, `kill preview`)
- Safari symlink in AppIndex
- Menu bar cache clear + AX walk budget
- QA session restore / paste_query keycodes

## Final status

- P0/P1/P2: **0 open** (2026-06-27 round)
- `scripts/test_unit.sh`: deterministic unit gate (skips `tag:integration` unless `LUMA_INTEGRATION_TESTS=1`)
- `swift test`: run after F-F-06–F-F-10
- Full smoke (`qa/final/run_smoke.sh`): **pass** (screenshots reviewed)
- Keystroke replay perf test: p95 < 30 ms (unit test)

## UI polish round (2026-07-02)

Follow-up to `qa/final-ui-acceptance-20260701-122711/report.md`. Code + docs aligned; see report **Remediation** section.

| Area | Status |
|------|--------|
| Tab / action panel / Shift+Tab | Fixed |
| Empty query stale results | Fixed |
| Bare commands (`todo`, `word review`, `s new`, `app top`) | Fixed |
| Detail layout cropping | Fixed (scroll toolbars, truncation) |
| Detail search vs module focus | Fixed (read-only placeholder, Esc restores query) |
| Single-char global search | Fixed (no fan-out, hint) |
| `help <trigger>` discoverability | Fixed (hints, empty states, module help lines) |
| Permissions / setup home rows | Fixed |
| List keyboard focus | Fixed (list accepts focus; ↑↓ from search or list; typing forwards to search) |
| Edit shortcuts app-wide | Fixed (`LumaStandardEditShortcuts`) |
| i18n (en + zh-Hans) | Fixed (`L10n`, `L10nStrings.json`, Settings → Language) |
| Translate replace selection | Fixed (`AXService.replaceSelectedText`, detail button) |
| Onboarding wizard | Fixed (`OnboardingWizardDetailView`, first-launch auto-present) |

Re-verify with `./scripts/build_app.sh` and manual pass on Retina display.

## Known P3

- Browser Tabs module remains **default-off**; QA enables via `enabledModules` in prep script.
- `tab` smoke may show duplicate GitHub rows when prep opens multiple Safari tabs.
- Automation permission required per browser (Safari granted on this machine).
