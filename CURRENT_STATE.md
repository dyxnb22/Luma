# Current State

## Snapshot

- Date: 2026-07-06 19:05:36 HKT
- Workspace: `/Users/diaoyuxuan/Luma`
- Branch: `main`
- Build: `swift build` passed.
- Key tests: 3 of 4 requested commands passed; `swift test --filter LumaAppTests` failed in `launcherFlowHarnessReplaysQuery`.
- Luma process: no running Luma app process found; only Cursor Helper processes with the workspace label `Luma` appeared in `pgrep` / `ps` output.
- Diagnostics: `~/Library/Logs/Luma/latency-report.json` exists; `diagnostics.json` and `crash-log.txt` are missing. Three `Luma-*.ips` crash reports exist for 2026-07-06.

## Git Status

Current branch from `git branch --show-current`:

```text
main
```

Current short status from `git status --short`:

```text
 M Sources/LumaApp/Launcher/LauncherListView.swift
 M scripts/scan_appkit_executor_risk.sh
```

Name/status from `git diff --name-status`:

```text
M	Sources/LumaApp/Launcher/LauncherListView.swift
M	scripts/scan_appkit_executor_risk.sh
```

Diff stat from `git diff --stat`:

```text
 Sources/LumaApp/Launcher/LauncherListView.swift |  4 +-
 scripts/scan_appkit_executor_risk.sh            | 69 +++++++++++++++++--------
 2 files changed, 50 insertions(+), 23 deletions(-)
```

Observed diff summary:

- `Sources/LumaApp/Launcher/LauncherListView.swift`: `acceptsFirstResponder` changed from checking `!rows.isEmpty` to checking `!LauncherListRows.selectableItems(from: rows).isEmpty`.
- `scripts/scan_appkit_executor_risk.sh`: warn-only scan for `@MainActor NSObject target/action entrypoints` changed from an `rg`/line-loop approach to an embedded `awk` state machine that tracks `@MainActor` class/actor scopes and warns on unbridged `@objc` functions.

No files were staged or committed during this snapshot.

## Build Result

Command:

```bash
swift build
```

Result: passed.

Key output:

```text
Building for debugging...
Build complete! (0.32s)
```

## Test Results

| Command | Result | Notes |
| --- | --- | --- |
| `swift test --filter LumaAppTests` | Failed | Exit code 1. SwiftPM first waited for another SwiftPM process using `.build` because the requested checks were run in parallel. 67 tests ran; `launcherFlowHarnessReplaysQuery` failed at `LauncherFlowHarness.swift:101:5` with `Expectation failed: ((harness.lastSnapshot?.items.isEmpty ?? true) -> true) == false`. This looks like a functional/snapshot-result failure from the test output. |
| `swift test --filter LumaCoreTests` | Passed | 317 tests in 1 suite passed after 4.186 seconds. Output included skipped machine/environment-sensitive tests such as Safari app scanner/global search checks, but the command passed. |
| `swift test --filter LumaModulesTests.simulatedUserTouchesEveryRequestedFeature` | Passed | 1 test passed after 0.046 seconds. SwiftPM first waited for another SwiftPM process using `.build`. |
| `swift test --filter LumaModulesTests.appsModuleTopTargetedQueryStaysUnderBudget` | Passed | 1 test passed after 0.116 seconds. SwiftPM first waited for another SwiftPM process using `.build`; no performance budget failure was reported. |

## Logs And Diagnostics

Checked paths:

| Path | Exists | Modified | Size | Notes |
| --- | --- | --- | --- | --- |
| `~/Library/Logs/Luma/` | Yes | 2026-07-06 11:45:31 HKT | 96 bytes | Directory contains only `latency-report.json` in `ls -l` output. |
| `~/Library/Logs/Luma/diagnostics.json` | No | n/a | n/a | Missing. This limits access to the structured diagnostics payload described in docs. |
| `~/Library/Logs/Luma/crash-log.txt` | No | n/a | n/a | Missing. |
| `~/Library/Logs/Luma/latency-report.json` | Yes | 2026-07-06 11:45:31 HKT | 2047 bytes | JSON file exists. |
| `~/Library/Logs/DiagnosticReports/Luma-2026-07-06-115651.ips` | Yes | 2026-07-06 11:56:51 HKT | 26201 bytes | Crash report for `app.luma`, version `0.1.0`, incident `6BF83B62-03D1-40C1-BF28-25EECB75AB1B`; exception `EXC_BAD_ACCESS`, signal `SIGSEGV`, faulting thread 0. |
| `~/Library/Logs/DiagnosticReports/Luma-2026-07-06-165734.ips` | Yes | 2026-07-06 16:57:34 HKT | 23766 bytes | Crash report for `app.luma`, version `0.1.0`, incident `0B466691-A042-4E51-9556-25C5C8ED7F7E`; exception `EXC_CRASH`, signal `SIGABRT`, faulting thread 0. |
| `~/Library/Logs/DiagnosticReports/Luma-2026-07-06-184548.ips` | Yes | 2026-07-06 18:45:49 HKT | 21320 bytes | Crash report for `app.luma`, version `0.1.0`, incident `871531A4-248E-4EF2-9148-E77D1F35F4D3`; exception `EXC_BAD_ACCESS`, signal `SIGSEGV`, faulting thread 0. |

`latency-report.json` key fields:

```text
generatedAt: 2026-07-06T03:45:31Z
hotkeyP95Milliseconds: 8344.972014427185
keystrokeP95Milliseconds: 19.132458
combinedP95Milliseconds: 8207.147002220154
keystrokeSampleCount: 57
hotkeySampleCount: 71
```

The latency report includes several hotkey samples around 8.1-8.6 seconds and keystroke samples mostly in the low milliseconds with p95 about 19.13 ms.

## Runtime State

Command:

```bash
pgrep -fl Luma || true
```

Output showed only Cursor Helper plugin processes whose command lines include the workspace label `Luma`, for example:

```text
27083 Cursor Helper (Plugin): extension-host (retrieval) Luma [1-9] ...
27084 Cursor Helper (Plugin): extension-host (agent-exec) Luma [1-10] ...
33241 Cursor Helper (Plugin): extension-host (user) Luma [1-2] ...
```

Additional `ps -axo pid=,comm=,args= | awk 'tolower($0) ~ /luma/ {print}'` output showed only the current `ps`/`awk` command itself and the same Cursor Helper processes. No process resembling `Luma.app/Contents/MacOS/Luma` was present. Based on these commands, no true Luma app process was running at snapshot time.

## Documentation Signals

README facts:

- `README.md` says to build and restart Luma with `./scripts/build_app.sh`.
- `README.md` says `build_app.sh` stops any old Luma process, builds and signs the app, then opens the new build so Command+Space is registered again.
- `README.md` says `./scripts/build_app.sh --no-restart` builds without running Luma.
- `README.md` says a `.app` bundle keeps bundle identifier `app.luma`, and signing with `Luma Local Development`, Apple Development, or Developer ID keeps Accessibility trust more stable than ad-hoc signing.
- `README.md` says `./scripts/repair_accessibility_permission.sh` resets stale TCC records when Accessibility appears enabled in System Settings but Luma still shows the permission banner.
- `README.md` says the LaunchAgent points at `build/Luma.app/Contents/MacOS/Luma` and restarts Luma after crashes while allowing normal Quit.

Engineering docs facts:

- `docs/ENGINEERING.md` says `DiagnosticsExport` writes redacted local JSON to `~/Library/Logs/Luma/diagnostics.json` with `platform`, `modules`, `permissions`, `recentErrors`, and `corruptConfigFiles`.
- `docs/ENGINEERING.md` says diagnostics export is triggered via `cmd export-diagnostics` through `HostClient.exportDiagnostics` and includes `CrashLogBuffer` breadcrumbs plus latency p95.
- `docs/ENGINEERING.md` says `cmd doctor` is fed by corrupt JSON quarantine information.
- `docs/ENGINEERING.md` says Accessibility permission is lazy: show the banner only on AX-dependent surfaces or after user interaction with Open Apps window controls.
- `docs/ENGINEERING.md` says disabled or permission-blocked modules return diagnostic rows, not silent empty results.
- `docs/ENGINEERING.md` says platform actions such as paste, focus, insert, open URL, and window layout should propagate errors from platform clients and must not report success when the platform call no-ops.

QA docs facts:

- `docs/QA.md` includes manual main-path checks for Cmd+Space show/hide, rapid toggle, Esc from home/search/detail/action panel, light/dark toggle, Notes detail/home scroll, and each MVP module bare prefix plus Return.
- `docs/QA.md` lists `./scripts/build_app.sh` and `./scripts/build_app.sh --no-restart` as local app build commands.
- `docs/QA.md` says scripted smoke includes `swift test --filter LumaModulesTests.simulatedUserTouchesEveryRequestedFeature` and `swift test --filter LumaModulesTests.simulateLauncherInputBoxFlows`.
- `docs/QA.md` says `cmd doctor` reports hotkey registration, corrupt config files, and latency p95 budget.
- `docs/QA.md` permission checks include no AX banner for empty home/ordinary app search when Accessibility is denied, AX banner on AX-dependent surfaces, actionable Browser Tabs Automation denial, and actionable Todo/EventKit denial.
- `docs/QA.md` says release checklist includes `swift test`, `./scripts/build_app.sh --no-restart`, DMG verification, fresh-machine Gatekeeper launch, and manual checks for hotkey, permissions, Browser Tabs Automation, Todo/EventKit, and AX-dependent modules.

## User-Reported Symptoms

Current subjective symptoms reported by the user:

- "这个软件一团糟"
- "基本功能都用不了"
- "之前做的几轮审查，让项目越来越乱"

These are recorded as subjective reports only and need later reproduction/confirmation. No specific unverified bug details are inferred here.

## Existing Uncommitted Changes

The workspace had uncommitted changes before `CURRENT_STATE.md` was created:

- `Sources/LumaApp/Launcher/LauncherListView.swift`: modified. Diff changes `acceptsFirstResponder` so the list accepts first responder only when `LauncherListRows.selectableItems(from: rows)` is non-empty, instead of when any row exists. Source unknown, possibly user or previous tool changes.
- `scripts/scan_appkit_executor_risk.sh`: modified. Diff replaces a simple file/line `rg` loop for `@MainActor` `@objc` target/action warnings with an embedded `awk` scanner that tracks class/actor scopes and bridge markers. Source unknown, possibly user or previous tool changes.

This task created `CURRENT_STATE.md`. No existing uncommitted changes were reverted, cleaned, staged, or committed.

## Risks For Next Phase

- `swift build` passing does not prove the launched `.app` is usable; the docs distinguish package build from `./scripts/build_app.sh`, signing, restart, hotkey registration, and permissions.
- One representative app test group currently fails (`launcherFlowHarnessReplaysQuery`), so green core/module tests do not cover all launcher behavior.
- `diagnostics.json` and `crash-log.txt` are missing, limiting structured diagnosis from the app's own diagnostics export path.
- Existing `.ips` crash reports show real Luma crashes on 2026-07-06, including `SIGSEGV` and `SIGABRT`.
- `latency-report.json` shows hotkey p95 around 8.3 seconds, so perceived basic interaction latency may be a major investigation area.
- Accessibility, Automation, EventKit/Todo, signing, bundle identity, TCC records, and LaunchAgent state may affect whether basic functions work.
- The workspace already has uncommitted changes from unknown origin; later work must not overwrite or reformat them casually.
- Tests were run with SwiftPM contention from parallel commands, although the final pass/fail results were still produced.

## Recommended Next Step

Proceed with an architecture fact map and user-flow trace before any broad refactor. Focus first on how launch/signing/permissions, hotkey show/hide, query dispatch, diagnostics export, and the launcher snapshot pipeline connect in the current code, then use that map to plan targeted stabilization work.
