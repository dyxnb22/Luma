# Usability Triage

## Scope

This is the Phase 5 triage artifact for Luma. It breaks the user-reported "basic functions do not work" feedback into reproducible, attributable, prioritized issues. It is not a refactor plan, not a fix list, and does not modify Swift source or tests.

## Inputs

Read before triage:

- `CURRENT_STATE.md`
- `ARCHITECTURE_MAP.md`
- `MODULE_MATRIX.md`
- `PRODUCT_FLOWS.md`
- `CONTRACTS.md`
- `docs/QA.md`
- `docs/ENGINEERING.md`
- `docs/MODULES.md`
- `README.md`

Commands run in this phase:

- `git status --short`
- `swift build`
- `swift test --filter LumaAppTests`
- `swift test --filter LumaModulesTests.simulatedUserTouchesEveryRequestedFeature`
- `swift test --filter LumaModulesTests.appsModuleTopTargetedQueryStaysUnderBudget`
- `swift test --filter AppKitExecutor`
- `swift test --filter LauncherFlowHarness`
- `swift test --filter PermissionBanner`
- `swift test --filter CommandsModuleDoctor`
- `pgrep -fl Luma || true`
- `ls -la ~/Library/Logs/Luma/`
- `ls -la ~/Library/Application\ Support/Luma/`
- `ls -lt ~/Library/Logs/DiagnosticReports/Luma-*.ips 2>/dev/null | head`
- `jq`/`tail`/small read-only Python probes for `latency-report.json`, `crash-log.txt`, and `.ips` summaries.

Luma was not started or restarted in this phase.

## Evidence Levels

- Confirmed: Current commands, tests, logs, or code facts directly support the finding.
- Previously Observed: Phase 0-4 recorded the finding, but this phase did not reproduce it.
- User-Reported: User subjective feedback exists, but no specific reproduction is confirmed.
- Inferred Risk: Code/contract deviation implies risk, but no direct runtime reproduction exists.
- Unknown: Information is insufficient.

## Current Snapshot

Git status:

```text
 M Sources/LumaApp/Launcher/LauncherListView.swift
 M scripts/scan_appkit_executor_risk.sh
?? ARCHITECTURE_MAP.md
?? CONTRACTS.md
?? CURRENT_STATE.md
?? MODULE_MATRIX.md
?? PRODUCT_FLOWS.md
```

The pre-existing source/script modifications were not edited, reverted, staged, or committed. This phase only adds this triage document.

Build and tests:

| Command | Result | Notes |
| --- | --- | --- |
| `swift build` | Passed | Build completed. Compiler emitted AppKit/MainActor warnings in `ClipboardDetailView.swift` and `LauncherListView.swift`, including nonisolated overrides touching main-actor AppKit members and sending `NSEvent` across actor isolation. |
| `swift test --filter LumaAppTests` | Passed now | 67 tests passed. Phase 0's `launcherFlowHarnessReplaysQuery` failure did not reproduce in this run. |
| `swift test --filter LumaModulesTests.simulatedUserTouchesEveryRequestedFeature` | Passed | 1 test passed. |
| `swift test --filter LumaModulesTests.appsModuleTopTargetedQueryStaysUnderBudget` | Passed | 1 test passed. |
| `swift test --filter AppKitExecutor` | Passed | 2 tests passed. |
| `swift test --filter LauncherFlowHarness` | Passed now | 2 tests passed, including `launcherFlowHarnessReplaysQuery`. |
| `swift test --filter PermissionBanner` | Passed | 2 tests passed. |
| `swift test --filter CommandsModuleDoctor` | Passed | 1 test passed. |

Runtime and logs:

- `pgrep -fl Luma` showed only Cursor Helper processes with workspace label `Luma`; no true `Luma.app/Contents/MacOS/Luma` process was identified.
- `~/Library/Logs/Luma/` exists and contains `latency-report.json` only. `~/Library/Logs/Luma/diagnostics.json` is missing.
- `~/Library/Logs/Luma/latency-report.json`: `generatedAt=2026-07-06T03:45:31Z`, `hotkeyP95Milliseconds=8344.972014427185`, `keystrokeP95Milliseconds=19.132458`, `combinedP95Milliseconds=8207.147002220154`, `hotkeySampleCount=71`, `keystrokeSampleCount=57`.
- `~/Library/Application Support/Luma/crash-log.txt` exists. Tail shows two redacted breadcrumbs from 2026-07-06T10:12Z: `note saved ~/<redacted>` and `query=<redacted> clipboard=<redacted>`.
- `~/Library/Application Support/Luma/clipboard-history.json` is about 37.9 MB and has a sibling `clipboard-history.json.corrupt-1782301078.bak`.
- Three `Luma-*.ips` reports exist for 2026-07-06.
  - 2026-07-06 18:45:48 HKT: `EXC_BAD_ACCESS`, `SIGSEGV`, faulting thread 0. Top frames include Swift executor checking and `@objc LauncherHomeGuidePane.tableView(_:shouldSelectRow:)`.
  - 2026-07-06 16:57:34 HKT: `EXC_CRASH`, `SIGABRT`, faulting thread 0. Top frames include Objective-C fatal lookup / `swift_getObjectType`.
  - 2026-07-06 11:56:51 HKT: `EXC_BAD_ACCESS`, `SIGSEGV`, faulting thread 0. Top frames include Swift executor checking and `@objc NotesMindMapView.isFlipped.getter`.

Key phase facts carried forward:

- Commands is default-off, so `cmd doctor` and `cmd export-diagnostics` are not reachable on a fresh default install unless Commands is enabled.
- `HotkeyConfig.save()` is a no-op.
- Menu bar Show calls `windowController.show()` directly, bypassing `showFromCarbonHotkey()`'s hidden-only guard and Carbon-show debounce.
- `ConfigCorruptionRegistry` is process-memory only.
- `JSONConfigPersistence.load` decode failures quarantine and record corruption, but read failures silently return fallback.
- Diagnostics payload fields for platform/modules/permissions can be empty unless passed explicit population sources.

## Symptom Matrix

| ID | Symptom | Evidence Level | Repro Status | Related Flow | Contract IDs | Likely Owner / Module | Test Coverage | Priority | Blocks MVP? | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| S-001 | Hotkey does not summon launcher | User-Reported | Needs user confirmation | F2, F3 | C-APPKIT-002, C-UI-001, C-ASYNC-002, C-TEST-001 | HotkeyController, LauncherWindowController | Hotkey/AppKit tests pass; no live app manual repro | P0 | Unknown | No Luma process is running now, which would make hotkey impossible, but user did not give exact repro. |
| S-002 | Hotkey summon is very slow | Confirmed | Previously observed via latency log | F3, F4 | C-HOT-002, C-ASYNC-003, C-TEST-001 | LauncherWindowController, HomeLatencyTracker, OpenAppsHomeProvider | No failing test; latency file is strong runtime evidence | P0 | Yes | Hotkey p95 about 8.3s vs 50/80 ms target/ceiling. |
| S-003 | Menu bar Show cannot open or behaves inconsistently | Inferred Risk | Needs user confirmation | F3, F12 | C-UI-001, C-ASYNC-002, C-TEST-001 | MenuBarController, LauncherWindowController | Carbon/menu path tests partial | P1 | Unknown | Code path bypasses Carbon guard/debounce; no runtime failure reproduced. |
| S-004 | Launcher opens but has no results / blank state | User-Reported | Needs user confirmation | F4, F5, F6, F7 | C-HOT-004, C-HOT-005, C-FAIL-002, C-UI-003 | LauncherRootController, QueryDispatcher, ModuleHost | Harness tests pass now | P0 | Unknown | Could be app not running, home blank, disabled modules, cold caches, or query gating. |
| S-005 | Empty query home abnormal | Previously Observed | Not reproduced now | F4 | C-UI-003, C-ASYNC-003, C-CACHE-002, C-TEST-001 | LauncherHomeCoordinator, OpenAppsHomeProvider | `emptyQueryHomeGuideHasRows` passed | P1 | Partial | Phase docs identify home/hotkey latency risk; no current blank-home repro. |
| S-006 | Query input returns no results | User-Reported | Needs user confirmation | F5, F6 | C-HOT-003, C-HOT-004, C-HOT-005, C-FAIL-005 | QueryView, LauncherViewModel, QueryDispatcher | `launcherFlowHarnessReplaysQuery` passed now | P0 | Unknown | Global search requires at least 2 chars; unprefixed non-contributing modules may correctly return none. |
| S-007 | Global search is slow | Inferred Risk | Not reproduced now | F6 | C-HOT-002, C-HOT-003, C-CACHE-001, C-TEST-001 | QueryDispatcher, Apps/Quicklinks/Clipboard | App top perf passed; keystroke p95 19 ms | P1 | Partial | Hotkey latency is confirmed; keystroke latency is currently within budget. |
| S-008 | Search results are wrong | User-Reported | Needs user confirmation | F6, F8 | C-HOT-003, C-UI-004, C-TEST-004 | QueryDispatcher, Ranker, LauncherContentCoordinator | No current failing ranking test | P1 | Unknown | Needs concrete query, expected row, actual row. |
| S-009 | Targeted module search has no results | Inferred Risk | Needs user confirmation | F7, F14 | C-HOT-004, C-FAIL-001, C-FAIL-002, C-FAIL-003, C-DEFAULT-005 | ModuleHost, per-module actors | User-flow simulation passed | P1 | Partial | Default-off modules return disabled rows; unconfigured Notes/Projects should show onboarding. |
| S-010 | Return does nothing | User-Reported | Needs user confirmation | F8, F9 | C-MODULE-002, C-FAIL-004, C-UI-004, C-TEST-001 | LauncherRootController, ActionExecutor | Action failure feedback test passes | P0 | Unknown | Could be no selected result, modules not ready, disabled module, or action failure. |
| S-011 | Return action fails with no feedback | Inferred Risk | Not reproduced now | F9, F13 | C-FAIL-004, C-FAIL-005, C-TEST-001 | ActionExecutor, LauncherRootController | `actionFailureSurfacesAccessibilityMessage` passed | P1 | Partial | Current test covers AX message; other platform failures need user case. |
| S-012 | Detail does not open | User-Reported | Needs user confirmation | F10 | C-DETAIL-002, C-DETAIL-005, C-FAIL-003, C-TEST-001 | LauncherDetailPresenter, ModuleDetailRegistry | Detail hierarchy/reuse tests pass in LumaAppTests | P1 | Partial | Disabled modules or modules without detail may correctly refuse detail. |
| S-013 | Detail back/return state is wrong | Inferred Risk | Needs user confirmation | F11, F12 | C-DETAIL-003, C-DETAIL-004, C-ASYNC-002, C-UI-005 | LauncherDetailLifecycleController, LauncherDetailExitPlanner | Detail/session tests pass now | P1 | Partial | Needs concrete module and Esc/back sequence. |
| S-014 | Hide and re-summon state is wrong | Inferred Risk | Needs user confirmation | F3, F12 | C-UI-001, C-ASYNC-002, C-ASYNC-003 | LauncherWindowController, LauncherRootController | Hotkey/double-fire/session tests pass | P1 | Partial | Generation guards exist; runtime still has crash reports on AppKit callbacks. |
| S-015 | Permission prompt is missing or appears at wrong time | Inferred Risk | Needs user confirmation | F13 | C-FAIL-001, C-FAIL-005, C-DEFAULT-002, C-TEST-001 | PermissionBannerController, PermissionResultBuilder | PermissionBanner tests pass | P1 | Partial | Lazy AX policy is intentional; EventKit/Automation use rows, not banner. |
| S-016 | Clipboard unusable | User-Reported | Needs user confirmation | F7, F9, F10, F13 | C-HOT-001, C-FAIL-001, C-PERSIST-001, C-CACHE-002 | ClipboardModule, ClipboardHistoryStore | Simulation passes; no live repro | P0 | Unknown | Large history and old corrupt backup are confirmed; current usability impact unconfirmed. |
| S-017 | Notes unusable | User-Reported | Needs user confirmation | F7, F10, F11, F15 | C-FAIL-003, C-PERSIST-001, C-DETAIL-003, C-APPKIT-001 | NotesModule, NotesDetailView | Notes-related tests pass in filtered suites | P0 | Unknown | Latest `.ips` includes a prior `NotesMindMapView.isFlipped` crash frame. |
| S-018 | Todo unusable | User-Reported | Needs user confirmation | F7, F9, F13 | C-FAIL-001, C-FAIL-003, C-DEFAULT-002 | TodoModule, RemindersService | Simulation passes; no live EventKit repro | P1 | Partial | EventKit denial should show actionable row; actual permission state not captured. |
| S-019 | Settings cannot open or save fails | Inferred Risk | Needs user confirmation | F9, F13, F15, F16 | C-FAIL-003, C-PERSIST-001, C-DEFAULT-005, C-TEST-001 | SettingsWindowController, ConfigurationStore | No full Settings open/save test run here | P0 | Unknown | Settings is important recovery path; `HotkeyConfig.save()` no-op is confirmed by design. |
| S-020 | `cmd doctor` / diagnostics are unreachable | Confirmed | Reproduced by configuration fact, not live UI | F7, F16 | C-DIAG-001, C-DEFAULT-005, C-TEST-001 | CommandsModule, ModuleHost | Commands doctor unit test passes when module is available | P0 | Yes | Commands module is default-off, cutting off fresh-install recovery commands. |
| S-021 | `diagnostics.json` is missing | Confirmed | Reproduced now | F16 | C-DIAG-001, C-DIAG-002, C-DIAG-004 | DiagnosticsExport, AppHostService, CommandsModule | Export payload unit path exists; file absent | P0 | Yes | No current structured diagnostic artifact exists in `~/Library/Logs/Luma/`. |
| S-022 | crash-log path/content is confusing or incomplete | Confirmed | Reproduced now | F16 | C-DIAG-003, C-DIAG-004 | CrashLogBuffer, CrashLogRecording | Crash log tests partial | P1 | Partial | File is under Application Support, not Logs; contains only redacted breadcrumbs, not `.ips` crash stacks. |
| S-023 | Restart loses state | Inferred Risk | Needs user confirmation | F1, F12, F15 | C-PERSIST-001, C-CACHE-001, C-TEST-001 | LauncherSessionStore, ConfigurationStore, per-module stores | Persistence tests partial | P1 | Unknown | `launcher-resume.json` exists; exact lost state not identified. |
| S-024 | Config corruption is invisible | Confirmed | Reproduced by code/log facts | F15, F16 | C-FAIL-006, C-PERSIST-002, C-DIAG-002 | JSONConfigPersistence, ConfigCorruptionRegistry, CommandsModule | JSONConfigPersistence tests cover decode quarantine | P1 | Partial | Registry is memory-only; clipboard quarantine is separate; read failure can silently fallback. |
| S-025 | Crash / app not running | Confirmed | Reproduced now | F1, F3, F12 | C-APPKIT-001, C-APPKIT-002, C-ASYNC-001, C-TEST-003 | AppKit views, LauncherWindowController, LauncherRootController | AppKit tests pass but runtime `.ips` exist | P0 | Yes | No true Luma process is running; three same-day crash reports exist. |
| S-026 | Tests and production path diverge, causing diagnostic distortion | Confirmed | Reproduced by phase docs and current green tests | F1, F6, F16 | C-TEST-001, C-TEST-004, C-REVIEW-001 | LauncherFlowHarness, AppCoordinator, ModuleBootstrapper | Harness now passes but does not exercise full startup | P1 | Partial | Current test green does not prove signed app launch/hotkey/diagnostics recovery. |

## Per-Symptom Details

### S-001 Hotkey does not summon launcher

**User Symptom**  
User has only given generalized feedback: "basic functions do not work." No exact hotkey reproduction was provided.

**Evidence**  
- Current `pgrep -fl Luma` found no true Luma app process, only Cursor Helper processes with the workspace label.
- README says `./scripts/build_app.sh` builds, signs, opens the app, and re-registers Command+Space.
- Hotkey registration failure records crash breadcrumbs and marks the menu bar warning state, but no live app was inspected.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F1 startup, F2 hotkey registration, F3 summon launcher.

**Contract Mapping**  
C-APPKIT-002, C-UI-001, C-ASYNC-002, C-TEST-001.

**Likely Owner**  
HotkeyController, AppCoordinator, LauncherWindowController.

**Existing Tests**  
`HotkeyRegister`, `HotkeyToggle`, `HotkeyDoubleFire`, `AppKitExecutor`; filtered LumaApp tests passed now.

**Priority**  
P0 because hotkey failure blocks the primary entry point.

**Blocks MVP**  
Unknown until the user confirms whether the app was running and whether menu bar Show works.

**Next Evidence To Collect**  
- User confirms whether Cmd+Space is completely silent or delayed.
- Confirm whether a built/signed Luma process stays running after `./scripts/build_app.sh`.
- If hotkey fails while app is running, capture `crash-log.txt` and latest `.ips`.

### S-002 Hotkey summon is very slow

**User Symptom**  
User has not specifically said "slow hotkey", but "basic functions do not work" can be explained by multi-second summon latency.

**Evidence**  
- `latency-report.json` confirms `hotkeyP95Milliseconds=8344.972014427185` and `combinedP95Milliseconds=8207.147002220154`.
- Engineering performance target is 50 ms p95 / 80 ms ceiling for hotkey to interactive/home.
- Keystroke p95 is 19.13 ms, so this points more to show/home path than typing path.

**Repro Status**  
Previously observed / Confirmed by existing latency artifact.

**Related Flows**  
F3 summon launcher, F4 empty query home.

**Contract Mapping**  
C-HOT-002, C-ASYNC-003, C-TEST-001.

**Likely Owner**  
LauncherWindowController, HomeLatencyTracker, LatencyTelemetry, OpenAppsHomeProvider.

**Existing Tests**  
No failing automated latency test in this run; performance file is runtime evidence.

**Priority**  
P0 because 8.3s p95 makes the main launcher feel unusable.

**Blocks MVP**  
Yes.

**Next Evidence To Collect**  
- Reproduce from a running signed app and record first-frame timestamps.
- Compare hotkey show with menu bar Show.
- Check whether Open Apps/home refresh counters spike during show.

### S-003 Menu bar Show cannot open or behaves inconsistently

**User Symptom**  
No specific menu bar Show report yet.

**Evidence**  
- PRODUCT_FLOWS records menu bar Show calls `windowController.show()` directly.
- This bypasses `showFromCarbonHotkey()`'s hidden-only guard and 120 ms Carbon debounce.
- No current command reproduced a user-visible Show failure.

**Repro Status**  
Needs user confirmation / Inferred risk.

**Related Flows**  
F3 summon launcher, F12 hide panel.

**Contract Mapping**  
C-UI-001, C-ASYNC-002, C-TEST-001.

**Likely Owner**  
MenuBarController, LauncherWindowController.

**Existing Tests**  
Carbon show/hide tests pass; no manual menu bar Show test was run.

**Priority**  
P1 because it is a fallback entry path if hotkey is broken.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- User confirms whether menu bar Show opens the panel.
- Try rapid menu bar Show and Cmd+Space sequencing while recording visibility state.

### S-004 Launcher opens but has no results / blank state

**User Symptom**  
User has not said whether the panel is blank, has guide only, or has no matches after query.

**Evidence**  
- Phase docs identify global vs targeted dispatch, query length gating, disabled module rows, and cold-module warming rows.
- `launcherFlowHarnessReplaysQuery` failed in Phase 0 but passed now.
- Current tests do not manually verify the launched signed app.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F4 empty home, F5 input query, F6 global search, F7 targeted search.

**Contract Mapping**  
C-HOT-004, C-HOT-005, C-FAIL-002, C-UI-003.

**Likely Owner**  
LauncherRootController, LauncherViewModel, QueryDispatcher, ModuleHost.

**Existing Tests**  
`LauncherFlowHarness`, `LumaAppTests`, and module simulation pass now.

**Priority**  
P0 if the panel is truly blank on normal app/search flows.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- User states exact visible panel content after launch.
- Capture queries `app safari`, `app`, `clip`, `n`, `todo`.
- Note whether query length is one character, a prefix, or global search.

### S-005 Empty query home abnormal

**User Symptom**  
No concrete empty-home description yet.

**Evidence**  
- Engineering contract says empty home should show Open Apps left and guide/detail right.
- `emptyQueryHomeGuideHasRows` passed now.
- Hotkey/home latency p95 is still confirmed high.

**Repro Status**  
Not reproduced now.

**Related Flows**  
F4 empty query home.

**Contract Mapping**  
C-UI-003, C-ASYNC-003, C-CACHE-002, C-TEST-001.

**Likely Owner**  
LauncherHomeCoordinator, OpenAppsHomeProvider, LauncherRootController.

**Existing Tests**  
`emptyQueryHomeGuideHasRows`, home snapshot/cache tests in LumaAppTests.

**Priority**  
P1 because abnormal home damages first-use trust, but current reproduction is absent.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Screenshot or description of empty home.
- Confirm whether Open Apps is empty, warming, or missing.
- Compare first show vs second show.

### S-006 Query input returns no results

**User Symptom**  
No exact query was provided.

**Evidence**  
- Global search requires at least 2 characters unless a command prefix is used.
- Global fan-out is limited to Apps, Quicklinks, Clipboard.
- `launcherFlowHarnessReplaysQuery` passed now after previously failing in Phase 0.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F5 input query, F6 global search.

**Contract Mapping**  
C-HOT-003, C-HOT-004, C-HOT-005, C-FAIL-005.

**Likely Owner**  
QueryView, LauncherViewModel, QueryDispatcher, Ranker.

**Existing Tests**  
`LauncherFlowHarness`, `QueryDispatcher`, module simulation.

**Priority**  
P0 if normal app search returns no results.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- Exact query text, expected result, actual rows.
- Test `app safari`, `saf`, `ql`, `clip text`.

### S-007 Global search is slow

**User Symptom**  
No specific slow-search report beyond generalized unusability.

**Evidence**  
- Runtime keystroke p95 is 19.13 ms, inside the documented 30/60 ms target/ceiling.
- App top targeted performance test passed.
- Hotkey p95 is slow, so perceived search slowness may actually be summon/home latency.

**Repro Status**  
Not reproduced now.

**Related Flows**  
F6 global search.

**Contract Mapping**  
C-HOT-002, C-HOT-003, C-CACHE-001, C-TEST-001.

**Likely Owner**  
QueryDispatcher, QuerySnapshotCache, AppsModule, QuicklinksModule, ClipboardModule.

**Existing Tests**  
`appsModuleTopTargetedQueryStaysUnderBudget` passed; broader keystroke tests not run in this phase.

**Priority**  
P1 because search speed is central, but current evidence points elsewhere.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Capture latency after typing representative global queries.
- Separate time-to-panel from time-to-results.

### S-008 Search results are wrong

**User Symptom**  
No concrete wrong-result example yet.

**Evidence**  
- Ranking/filtering are in QueryDispatcher/Ranker; no current test failure.
- Phase 3 listed possible prior harness factors including ranking filtering, but did not isolate root cause.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F6 global search, F8 selection.

**Contract Mapping**  
C-HOT-003, C-UI-004, C-TEST-004.

**Likely Owner**  
QueryDispatcher, Ranker, LauncherContentCoordinator.

**Existing Tests**  
Filtered launcher and module tests passed.

**Priority**  
P1.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- Exact query, expected top result, actual top result.
- Confirm whether result is missing, ranked too low, or wrong action target.

### S-009 Targeted module search has no results

**User Symptom**  
No exact module/query was provided.

**Evidence**  
- Default-off modules return disabled diagnostic rows, not actual module results.
- Notes/Projects require configured roots and should show onboarding rows.
- Targeted cold modules should emit warming rows.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F7 targeted module search, F14 module cold start.

**Contract Mapping**  
C-HOT-004, C-FAIL-001, C-FAIL-002, C-FAIL-003, C-DEFAULT-005.

**Likely Owner**  
ModuleHost, QueryDispatcher, per-module actors.

**Existing Tests**  
`simulatedUserTouchesEveryRequestedFeature` passed.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Identify exact module: Clipboard, Notes, Todo, Apps, etc.
- Record whether a disabled/configuration/permission row appears.

### S-010 Return does nothing

**User Symptom**  
No exact Return case was provided.

**Evidence**  
- PRODUCT_FLOWS says Return can no-op/status if modules are not ready, no result is selected, or action cannot run.
- Action failure feedback test passed.
- No live signed-app action was run.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F8 selection, F9 action execution.

**Contract Mapping**  
C-MODULE-002, C-FAIL-004, C-UI-004, C-TEST-001.

**Likely Owner**  
LauncherRootController, ActionExecutor, per-module `perform`.

**Existing Tests**  
`actionFailureSurfacesAccessibilityMessage`, module user-flow simulation.

**Priority**  
P0 when Return fails on Apps/Clipboard/Notes/Todo main path.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- Exact selected row and whether any status appears.
- Distinguish Return from Command+Return.

### S-011 Return action fails with no feedback

**User Symptom**  
No exact failure/no-feedback action was provided.

**Evidence**  
- Engineering contract requires platform action failures to surface status and keep panel open.
- `actionFailureSurfacesAccessibilityMessage` passed for one AX action class.
- Other platform failures were not reproduced.

**Repro Status**  
Not reproduced now / Inferred risk.

**Related Flows**  
F9 action execution, F13 permission failure.

**Contract Mapping**  
C-FAIL-004, C-FAIL-005, C-TEST-001.

**Likely Owner**  
ActionExecutor, LauncherRootController, platform clients.

**Existing Tests**  
Partial coverage for Accessibility failures.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Trigger failing paste/open/focus action and record status bar text.
- Capture `crash-log.txt` after failure.

### S-012 Detail does not open

**User Symptom**  
No module-specific detail failure was provided.

**Evidence**  
- Some modules intentionally have no registered detail.
- Disabled modules abort detail presentation with a status.
- Detail reuse/presenter tests passed now.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F10 open detail.

**Contract Mapping**  
C-DETAIL-002, C-DETAIL-005, C-FAIL-003, C-TEST-001.

**Likely Owner**  
LauncherDetailPresenter, ModuleDetailRegistry, LauncherContentCoordinator.

**Existing Tests**  
Detail hierarchy/reuse tests in LumaAppTests passed.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Exact module and entry path.
- Confirm if module is enabled and has registered detail.

### S-013 Detail back/return state is wrong

**User Symptom**  
No exact sequence was provided.

**Evidence**  
- Detail lifecycle is split across presenter, registry, lifecycle controller, and exit planner.
- Tests covering search-detail mode, reuse, and coordinator behavior passed.
- No manual live detail round-trip was run.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F11 back from detail, F12 hide panel.

**Contract Mapping**  
C-DETAIL-003, C-DETAIL-004, C-ASYNC-002, C-UI-005.

**Likely Owner**  
LauncherDetailLifecycleController, LauncherDetailExitPlanner, LauncherContentCoordinator.

**Existing Tests**  
LumaAppTests detail/session tests passed.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Module, query, detail entry, Esc/back/hide sequence.
- Record whether search field becomes editable afterward.

### S-014 Hide and re-summon state is wrong

**User Symptom**  
No exact hide/reopen failure was provided.

**Evidence**  
- Generation guards and cancel paths are documented and tested.
- Crash reports show AppKit callback/executor crashes from earlier runs, so UI lifecycle remains a credible risk area.

**Repro Status**  
Needs user confirmation / Inferred risk.

**Related Flows**  
F3 summon launcher, F12 hide panel.

**Contract Mapping**  
C-UI-001, C-ASYNC-002, C-ASYNC-003.

**Likely Owner**  
LauncherWindowController, LauncherRootController.

**Existing Tests**  
Hotkey, panel visibility, snapshot cancellation tests passed now.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Repeat Cmd+Space show/hide and Esc hide loops on running app.
- Check if panel is hidden, transparent, unfocused, or stale.

### S-015 Permission prompt is missing or appears at wrong time

**User Symptom**  
No exact permission prompt symptom was provided.

**Evidence**  
- AX banner is intentionally lazy and should not appear on empty home/plain app search.
- Todo/EventKit and Browser/Automation surface as diagnostic rows, not the AX banner.
- PermissionBanner tests passed.

**Repro Status**  
Needs user confirmation / Inferred risk.

**Related Flows**  
F13 permission failure.

**Contract Mapping**  
C-FAIL-001, C-FAIL-005, C-DEFAULT-002, C-TEST-001.

**Likely Owner**  
PermissionBannerController, PermissionResultBuilder, TodoModule, BrowserTabsModule.

**Existing Tests**  
`PermissionBanner` filtered tests passed.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Identify which permission and which surface.
- Record whether banner, row, or status appears.

### S-016 Clipboard unusable

**User Symptom**  
No concrete Clipboard behavior was provided.

**Evidence**  
- Clipboard is default-on and P0.
- Application Support contains a large `clipboard-history.json` and an older corrupt backup.
- Clipboard paste-directly requires Accessibility; copy should work without AX.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F7 targeted search, F9 action, F10 detail, F13 permission.

**Contract Mapping**  
C-HOT-001, C-FAIL-001, C-PERSIST-001, C-CACHE-002.

**Likely Owner**  
ClipboardModule, ClipboardHistoryStore, ClipboardDetailView.

**Existing Tests**  
Module simulation passed; specific Clipboard tests were not run in this phase.

**Priority**  
P0 because Clipboard is MVP default-on.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- Test `clip`, clipboard search, Return copy, secondary paste.
- Note history size and whether detail loads slowly or empty.

### S-017 Notes unusable

**User Symptom**  
No concrete Notes behavior was provided.

**Evidence**  
- Notes is default-on and should show onboarding if root is unset.
- Latest same-day `.ips` includes an earlier faulting frame `@objc NotesMindMapView.isFlipped.getter`.
- No current Notes repro or failing filtered test.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F7 targeted search, F10 detail, F11 return, F15 config corruption.

**Contract Mapping**  
C-FAIL-003, C-PERSIST-001, C-DETAIL-003, C-APPKIT-001.

**Likely Owner**  
NotesModule, NotesDetailView, NotesMindMapView.

**Existing Tests**  
Filtered app tests passed; specific full Notes manual smoke not run.

**Priority**  
P0 because Notes is MVP default-on and has crash-adjacent evidence.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- Test `n`, Notes detail, configured root state, create/open note.
- If crash recurs, inspect latest `.ips` faulting thread.

### S-018 Todo unusable

**User Symptom**  
No concrete Todo behavior was provided.

**Evidence**  
- Todo is default-on and depends on EventKit/Reminders.
- Permission denied should show an actionable row.
- No current live EventKit permission state was captured.

**Repro Status**  
Needs user confirmation.

**Related Flows**  
F7 targeted search, F9 action, F13 permission.

**Contract Mapping**  
C-FAIL-001, C-FAIL-003, C-DEFAULT-002.

**Likely Owner**  
TodoModule, RemindersService.

**Existing Tests**  
Module simulation passed.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Test `todo`/`t`, create reminder, complete/uncomplete.
- Record Reminders permission state and row text.

### S-019 Settings cannot open or save fails

**User Symptom**  
No exact Settings failure was provided.

**Evidence**  
- Menu bar Settings and Commands `settings` route exist.
- Commands is default-off, so command-based Settings may be unavailable on fresh install.
- `HotkeyConfig.save()` is a no-op by code fact, so hotkey edits cannot persist by design.
- No full Settings open/save test was run in this phase.

**Repro Status**  
Needs user confirmation / Inferred risk.

**Related Flows**  
F9 action, F13 permission, F15 config, F16 diagnostics.

**Contract Mapping**  
C-FAIL-003, C-PERSIST-001, C-DEFAULT-005, C-TEST-001.

**Likely Owner**  
SettingsWindowController, SettingsSwiftUIView, ConfigurationStore, CommandsModule.

**Existing Tests**  
No end-to-end Settings window test confirmed.

**Priority**  
P0 if Settings is needed to recover modules/permissions and cannot open.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- User confirms menu bar Settings opens.
- Test saving Notes root, enabled modules, Clipboard settings, and restart persistence.

### S-020 `cmd doctor` / diagnostics are unreachable

**User Symptom**  
No specific `cmd doctor` report was provided, but Phase 2/3 established reachability issue.

**Evidence**  
- Commands manifest is default-off.
- PRODUCT_FLOWS says `cmd export-diagnostics` requires Commands enabled because disabled modules are unreachable via targeted dispatch.
- `CommandsModuleDoctor` unit test passes when Commands is available, so logic exists but may be cut off by defaults.

**Repro Status**  
Confirmed by configuration/code fact; not manually run in UI.

**Related Flows**  
F7 targeted search, F16 diagnostics export.

**Contract Mapping**  
C-DIAG-001, C-DEFAULT-005, C-TEST-001.

**Likely Owner**  
CommandsModule, ModuleRegistry/defaults, ModuleHost.

**Existing Tests**  
`commandsDoctorUsesInjectedPlatformClients` passed.

**Priority**  
P0 because it blocks recovery and support diagnosis.

**Blocks MVP**  
Yes.

**Next Evidence To Collect**  
- On fresh/default config, query `cmd doctor` and record row.
- Enable Commands and compare `cmd doctor` / `cmd export-diagnostics`.

### S-021 `diagnostics.json` is missing

**User Symptom**  
No explicit report, but missing diagnostics blocks this phase's evidence.

**Evidence**  
- `~/Library/Logs/Luma/diagnostics.json` is missing now.
- `~/Library/Logs/Luma/` contains only `latency-report.json`.
- Export path exists in code via `DiagnosticsExport.exportToLogsDirectory`.

**Repro Status**  
Reproduced now.

**Related Flows**  
F16 diagnostics export.

**Contract Mapping**  
C-DIAG-001, C-DIAG-002, C-DIAG-004.

**Likely Owner**  
DiagnosticsExport, AppHostService, CommandsModule.

**Existing Tests**  
Command payload/export host stubs exist; no current file generated.

**Priority**  
P0 because missing diagnostics blocks recovery evidence.

**Blocks MVP**  
Yes.

**Next Evidence To Collect**  
- Attempt explicit `cmd export-diagnostics` after Commands is reachable.
- Verify generated fields: platform, modules, permissions, recentErrors, corruptConfigFiles.

### S-022 crash-log path/content is confusing or incomplete

**User Symptom**  
No explicit crash-log complaint.

**Evidence**  
- `crash-log.txt` exists at `~/Library/Application Support/Luma/crash-log.txt`, not `~/Library/Logs/Luma/`.
- It contains redacted breadcrumbs, not full crash reports.
- `.ips` crash reports are separate under `~/Library/Logs/DiagnosticReports/`.

**Repro Status**  
Reproduced now.

**Related Flows**  
F16 diagnostics/export and breadcrumbs.

**Contract Mapping**  
C-DIAG-003, C-DIAG-004.

**Likely Owner**  
CrashLogBuffer, CrashLogRecording, diagnostics docs.

**Existing Tests**  
Crash log redaction tests are referenced by repo inventory; not specifically run here.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- After a failed action or hotkey failure, confirm breadcrumb is appended.
- Confirm diagnostics export includes breadcrumbs.

### S-023 Restart loses state

**User Symptom**  
No exact lost state was provided.

**Evidence**  
- `launcher-resume.json` exists in Application Support.
- Several persistence paths exist: UserDefaults, JSON stores, Keychain, EventKit, Markdown roots.
- Phase facts note JSON read failures can silently fallback.

**Repro Status**  
Needs user confirmation / Inferred risk.

**Related Flows**  
F1 startup, F12 hide, F15 config corruption.

**Contract Mapping**  
C-PERSIST-001, C-CACHE-001, C-TEST-001.

**Likely Owner**  
LauncherSessionStore, ConfigurationStore, per-module stores.

**Existing Tests**  
JSON persistence tests exist; full signed-app restart persistence not run.

**Priority**  
P1.

**Blocks MVP**  
Unknown.

**Next Evidence To Collect**  
- Identify lost state: enabled modules, roots, query/session, Clipboard, Notes, Todo, Settings.
- Compare files before/after app restart.

### S-024 Config corruption is invisible

**User Symptom**  
No explicit corruption report.

**Evidence**  
- `ConfigCorruptionRegistry` is process-memory only.
- `JSONConfigPersistence` records decode corruption, but read failures silently fallback.
- Clipboard has its own `.corrupt-*.bak` backup not necessarily visible to `cmd doctor`.
- Commands is default-off, so doctor visibility is also gated.

**Repro Status**  
Confirmed by code/phase facts.

**Related Flows**  
F15 config corruption, F16 diagnostics.

**Contract Mapping**  
C-FAIL-006, C-PERSIST-002, C-DIAG-002.

**Likely Owner**  
JSONConfigPersistence, ConfigCorruptionRegistry, ClipboardHistoryStore, CommandsModule.

**Existing Tests**  
`JSONConfigPersistenceTests` cover decode quarantine; broader diagnostics visibility not proven.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Corrupt representative files and run doctor/export after enabling Commands.
- Check whether clipboard corrupt backup appears anywhere.

### S-025 Crash / app not running

**User Symptom**  
"Basic functions do not work" is compatible with the app not running or crashing.

**Evidence**  
- No true Luma process found now.
- Three same-day `.ips` reports exist for app `app.luma`.
- Latest crash: `EXC_BAD_ACCESS`/`SIGSEGV`, faulting thread 0, Swift executor check frames, `LauncherHomeGuidePane.tableView(_:shouldSelectRow:)`.
- Earlier crash: `NotesMindMapView.isFlipped.getter`.
- `swift build` emits AppKit/MainActor warnings in view override code.

**Repro Status**  
Reproduced now for "not running"; crashes previously observed in `.ips`.

**Related Flows**  
F1 startup, F3 summon, F12 hide.

**Contract Mapping**  
C-APPKIT-001, C-APPKIT-002, C-ASYNC-001, C-TEST-003.

**Likely Owner**  
AppKit view subclasses, LauncherWindowController, LauncherRootController, Notes detail/home views.

**Existing Tests**  
AppKitExecutor tests pass, but runtime `.ips` proves tests are not sufficient.

**Priority**  
P0.

**Blocks MVP**  
Yes.

**Next Evidence To Collect**  
- Launch via `./scripts/build_app.sh` and observe whether process stays running.
- Clear old `.ips`, reproduce crash, inspect new faulting thread.

### S-026 Tests and production path diverge, causing diagnostic distortion

**User Symptom**  
User reported prior reviews made the project "more chaotic"; green tests may not match real usability.

**Evidence**  
- Phase 3 says LauncherFlowHarness builds an independent ModuleHost/QueryDispatcher/ViewModel stack, not full AppCoordinator startup.
- `LumaAppTests` failed in Phase 0 but passed now.
- Current test green does not prove signed app launch, LaunchAgent, Carbon registration, TCC, menu bar, or diagnostics reachability.

**Repro Status**  
Confirmed by phase docs and current test/runtime contrast.

**Related Flows**  
F1 startup, F6 query, F16 diagnostics.

**Contract Mapping**  
C-TEST-001, C-TEST-004, C-REVIEW-001.

**Likely Owner**  
LauncherFlowHarness, AppCoordinator, ModuleBootstrapper, QA process.

**Existing Tests**  
Many filtered tests pass; full production parity is not covered.

**Priority**  
P1.

**Blocks MVP**  
Partial.

**Next Evidence To Collect**  
- Run signed-app smoke rather than only SwiftPM tests.
- Add recorded evidence for startup, hotkey, diagnostics, and permissions in future phases.

## MVP Blockers

- S-025: No true Luma process is currently running and three same-day `.ips` crashes exist.
- S-002: Hotkey p95 is about 8.3 seconds, far beyond the documented 50/80 ms budget.
- S-020: `cmd doctor` / `cmd export-diagnostics` recovery surface is gated by default-off Commands.
- S-021: `diagnostics.json` is missing, so structured support evidence is unavailable.
- S-024: Corruption visibility is incomplete: memory-only registry, split quarantine paths, and silent read fallback.
- S-026: Current green tests do not prove full startup/signed app/hotkey/diagnostics production parity.

Phase 0's `launcherFlowHarnessReplaysQuery` failure is now classified as Previously Observed, not a current reproduced blocker, because this phase's `LumaAppTests` and `LauncherFlowHarness` runs passed.

## Diagnostic Blockers

- `diagnostics.json` is missing from `~/Library/Logs/Luma/`.
- `cmd export-diagnostics` is default-off via the Commands module.
- `cmd doctor` is likewise default-off, limiting fresh-install recovery.
- `crash-log.txt` is under Application Support while diagnostics live under Logs, and the file is breadcrumbs only.
- `CrashLogBuffer.persist()` swallows write failures with `try?`.
- `ConfigCorruptionRegistry` is memory-only.
- Clipboard corruption uses a separate `.corrupt-*.bak` path.
- `JSONConfigPersistence` read failures can silently fallback.
- Launcher harness tests do not exercise full `AppCoordinator.start()` / signed app / Carbon / LaunchAgent paths.
- Diagnostics payload fields can be empty unless explicit population sources are passed.

## Test Coverage Gaps

- Full startup through `AppCoordinator.start()` and `ModuleBootstrapper`.
- Signed `.app` launch via `./scripts/build_app.sh` and process-stays-running verification.
- Hotkey registration failure with visible recovery state.
- Wall-clock hotkey-to-interactive/home latency regression.
- Menu bar Show parity with Carbon hotkey show.
- LauncherFlowHarness parity with production global-search configuration and startup warmup.
- Diagnostics reachability on fresh/default install.
- `cmd export-diagnostics` actual file generation and field population.
- Permission rows across AX, EventKit, Automation, and default-off modules.
- Settings open/save/restart persistence.
- Restart persistence for launcher session, enabled modules, Notes roots, Clipboard settings, and module stores.
- Runtime AppKit executor crash coverage matching the observed `.ips` callback frames.

## User Confirmation Needed

Launch / Hotkey:

- Is Cmd+Space completely unresponsive, or does Luma appear after several seconds?
- Is Luma visible in Activity Monitor / menu bar when this happens?
- Does menu bar Show open the launcher?

Search / Results:

- After opening, is the panel blank, guide-only, or showing rows?
- Does `app safari` return Safari?
- Does unprefixed `saf` return anything?
- Does `app` alone show Apps rows or a guide row?

Actions / Return:

- On which row does Return do nothing?
- Does a status message appear after Return?
- Does Command+Return behave differently?

Detail:

- Which detail fails: Clipboard, Notes, Todo, Translate, Snippets, Quicklinks?
- Does Esc return to home/results correctly?
- Is the search field editable after returning?

Modules:

- Clipboard: does `clip` open detail, and does Return copy?
- Notes: is a root configured, and does `n` open detail/onboarding?
- Todo: what is the Reminders permission state?

Permissions:

- Does Accessibility banner appear on plain home/app search?
- Does it appear on Snippet paste, Window Layouts, or Menu Bar Search?
- Do Todo/EventKit and Browser Automation denials show actionable rows?

Settings / Persistence:

- Can menu bar Settings open?
- Which setting fails to save?
- After restart, what exactly is lost: enabled modules, roots, session, Clipboard, Notes, Todo?

Diagnostics / Crash:

- Can `cmd doctor` be reached after enabling Commands?
- Can `cmd export-diagnostics` generate `~/Library/Logs/Luma/diagnostics.json`?
- Did a new `.ips` appear immediately after the unusable behavior?

## Suggested Evidence Collection Order

1. Confirm whether `./scripts/build_app.sh` launches Luma and the process stays running.
2. Confirm Cmd+Space behavior: no response vs delayed response vs crash.
3. If crash occurs, clear old `.ips`, reproduce once, and read the newest faulting thread.
4. Confirm menu bar Show and menu bar Settings as fallback paths.
5. Enable Commands if needed, then attempt `cmd doctor` and `cmd export-diagnostics`.
6. Reproduce `app`, `app safari`, `clip`, `n`, and `todo` with screenshots or row text.
7. For any failed Return action, collect selected row, status text, and `crash-log.txt` tail.
8. Compare failing live behavior against the smallest passing or failing automated test.
