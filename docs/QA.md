# Luma QA, Testing, And Release

This file replaces the older manual QA checklist, recorded QA brief, testing iteration notes, release notes, and QA summaries.

## Automated Gates

Run before merging launcher/module changes:

```bash
swift test
scripts/scan_appkit_executor_risk.sh
```

**SwiftPM tests alone are not sufficient for P0 release or post-P0 PRs** — signed-app smoke is required (Phase 9.8 gate; see § P0 MVP Smoke Gate below).

Targeted checks:

```bash
swift test --filter KeystrokeReplayPerformanceTests
swift test --filter AppsModuleTopQueryPerformanceTests
swift test --filter QueryDispatcher
swift test --filter LauncherSession
swift test --filter PanelSignals
swift test --filter QuerySnapshot
swift test --filter LauncherSnapshot
swift test --filter BackHome
swift test --filter DetailHierarchy
swift test --filter AppKitExecutor
swift test --filter LauncherListRowReuse
swift test --filter LauncherPanelExecutor
swift test --filter HotkeyToggle
swift test --filter LauncherPanelVisibilitySession
swift test --filter CancellationGeneration
```

## Swift 6 / AppKit smoke (manual)

After executor-boundary changes, clear `~/Library/Logs/DiagnosticReports/Luma-*.ips`, then verify:

- Signed app starts and stays running; the menu bar icon is present.
- Cmd+Space show/hide ×20 without crash
- Cmd+Space rapid toggle ×50 without panel stuck hidden
- Esc from home, search results, detail, and action panel
- System light/dark toggle with panel open
- Notes detail + home list scroll without crash
- Menu bar Show opens the panel when the hotkey path is unavailable.
- Launcher input accepts text, shows the empty-query home, and executes Return on a selected row.
- Apps P0 smoke: `app safari` returns a row and Return opens or focuses Safari.
- Clipboard P0 smoke: `clip` searches history and Return copies the selected item.
- Notes P0 smoke: bare `n` opens Notes detail/onboarding; `n new` creates through the configured root.
- Settings P0 smoke: menu bar Settings opens; saving a non-destructive setting persists after restart.
- Diagnostics P0 smoke: Doctor/export is reachable outside Commands default-off gating (menu bar or Settings recovery entry); export writes diagnostics; no new `Luma-*.ips` appears.

Parked/deferred and non-P0 modules are extended QA only. Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench/Capture, Commands user scripts, plus Core P1/conditional modules such as Snippets, Quicklinks, Translate, and Todo, must not be used as a P0 smoke gate unless they crash or corrupt the default P0 path.

See `docs/swift6-appkit-boundaries.md` for the full contract.

Build local app:

```bash
./scripts/build_app.sh
```

Build without restarting:

```bash
./scripts/build_app.sh --no-restart
```

Scripted smoke:

```bash
./scripts/run_p0_smokes.sh [path/to/Luma.app]
```

Terminable P0 gate (**recommended**): runs `LUMA_QA_APPS`, `LUMA_QA_CLIPBOARD`, `LUMA_QA_NOTES`, `LUMA_QA_SETTINGS`, and `LUMA_QA_EXPORT` with `LUMA_QA_AUTO_EXIT=1`, polling `~/Library/Logs/Luma/*-smoke.json` (and `diagnostics.json`). Exits 0/1. **Validated** on signed `build/Luma.app` (Phase 15–16, 2026-07-07; see `PHASE15_P2_EXECUTION_REPORT.md`, `P2_EXIT_SUMMARY.md`). Per-module manual commands remain below as fallback.

**`LUMA_QA_AUTO_EXIT=1`:** Set automatically by `run_p0_smokes.sh` (and intended for CI). Do **not** set on normal launch or manual smoke runs unless you want the app to terminate immediately after writing JSON.

Screenshot / drive-based full smoke:

```bash
./scripts/qa/run_full_smoke.sh
```

Prepare deterministic smoke data:

```bash
./scripts/qa/prep_smoke_env.sh
```

## P0 MVP Smoke Gate (Phase 9.8)

**Baseline commit:** `889ebd35` (`P0_EXIT_SUMMARY.md`). Re-run this gate before any P1/P2/P3 merge and before release tags.

### Prerequisites

**Recommended:** run the full terminable gate in one command after build:

```bash
swift build
./scripts/build_app.sh --no-restart
./scripts/run_p0_smokes.sh
```

Or step through prerequisites manually:

Record `.ips` count before smokes:

```bash
ls ~/Library/Logs/DiagnosticReports/Luma*.ips 2>/dev/null | wc -l
```

Check latency baseline (optional but recommended):

```bash
cat ~/Library/Logs/Luma/latency-report.json
# hotkeyP95Milliseconds must stay ≤ 1000 ms (P0 emergency ceiling)
```

### Core SwiftPM filters (P0)

```bash
swift test --filter AppsModuleTests
swift test --filter Clipboard
swift test --filter Notes
swift test --filter Settings
swift test --filter Config
swift test --filter Persistence
swift test --filter DiagnosticsExport
swift test --filter LauncherActionDispatch
```

### Signed-app env-gated smokes

Run **one env var at a time**; kill `Luma` between runs. Each writes JSON under `~/Library/Logs/Luma/`. **Do not** set these on normal launch. **Do not** set `LUMA_QA_AUTO_EXIT=1` for manual runs unless you intend the app to exit after writing JSON (the scripted runner sets it for you).

```bash
APP=build/Luma.app/Contents/MacOS/Luma

pkill -x Luma; LUMA_QA_EXPORT=1 "$APP"      # → diagnostics.json
pkill -x Luma; LUMA_QA_APPS=1 "$APP"        # → apps-smoke.json
pkill -x Luma; LUMA_QA_CLIPBOARD=1 "$APP"   # → clipboard-smoke.json
pkill -x Luma; LUMA_QA_NOTES=1 "$APP"       # → notes-smoke.json
pkill -x Luma; LUMA_QA_SETTINGS=1 "$APP"    # → settings-smoke.json
```

Wait ~12 s per run; confirm `pgrep -x Luma` shows the real app binary (not Cursor Helper).

**Config restore:** `LUMA_QA_NOTES` and `LUMA_QA_SETTINGS` briefly modify config (`notes.json`, UserDefaults). Smokes restore on success; if killed mid-run, verify `notes.json` root and Settings values manually.

### Gate checks (all required)

| Check | Pass criteria |
|-------|----------------|
| `.ips` | Count unchanged after all smokes |
| `diagnostics.json` | `platform`, `modules`, `permissions`, `breadcrumbs`, `recentErrors`, `latencyP95Milliseconds`, `crashLogPath`, `crashLogWriteStatus` present; `enabledModuleIDs` reflects real enabled set; `mvpCoreModuleStatus` lists Apps/Clipboard/Notes separately |
| `crashLogPath` | Points to `~/Library/Application Support/Luma/crash-log.txt` |
| Apps smoke | `launchResult: success` (or documented skip) |
| Clipboard smoke | `copySucceeded: true`, `bareClipOpensDetail: true` |
| Notes smoke | `createdFileExists`, `configRestored` / `diskRootMatchesBackup` |
| Settings smoke | `latencyHUDPersisted`, `latencyHUDRestored`, `clipboardMaxEntriesRestored` |
| Hotkey latency | `hotkeyP95Milliseconds` ≤ 1000 ms when `latency-report.json` exists |
| Recovery outside Commands | Menu bar **Run Doctor…** and **Export Diagnostics…** work with Commands default-off |

### Not P0 gate

Parked/deferred modules and Core P1/conditional modules (**Snippets, Quicklinks, Translate, Todo**, Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, Commands user scripts, Workbench/Capture) are extended QA only unless they crash or corrupt the default P0 path.

### Manual supplement (release tag)

After automated gate, manually verify: Cmd+Space show/hide ×20, menu bar Show, Esc/hide/reshow search input, menu bar Settings open.

## Review Lenses

- Functional correctness: feature works end to end and persists when expected.
- Keyboard-first quality: the flow is complete without a mouse.
- Visual stability: no flash, jump, width drift, overlap, clipping, or row jitter.
- Performance feel: first paint and keystrokes are immediate.
- Trust: permission denial, missing dependency, empty data, and failures are clear.
- Data safety: destructive actions confirm or offer an undo/status path.

## Core Manual Smoke

Launcher:

- Hotkey shows and hides the panel repeatedly without flash.
- First frame shows empty home without rebuilding Open Apps.
- Empty home shows Open Apps left and command guide right.
- Typing collapses to single-column results.
- Clearing query restores home.
- Esc steps through action panel/detail/results/home/close.
- Return runs primary action and hides fast for external actions.
- Tab or Command+K opens action panel.
- Command+Return runs first secondary action.
- Command+1...9 targets visible result/action rows.
- Search field remains editable after leaving detail.
- Multi-monitor placement uses the presentation screen and keeps the panel centered.
- Move cursor to an external display (or switch Space) while the panel stays open — panel repositions to the current presentation screen without manual hide/show.

Action feedback (stabilization):

- Trigger an action that requires Accessibility without granting permission (e.g. Snippet paste, clipboard paste-directly) — status bar shows permission message; panel stays open.
- Copy-to-clipboard success still shows brief status and delayed dismiss.

CJK IME:

- Enable Chinese Pinyin input; type several letters before selecting a candidate — results must not update until composition commits.

Action panel + query:

- Open action panel (Tab) on a result row; type in the search field — panel dismisses and results refresh for the new query.

Multi-monitor (manual):

- Show panel on display A; move cursor to display B; switch Space — panel recenters on the presentation screen while visible.

Performance:

- Hotkey -> interactive panel feels instant.
- Keystroke -> first paint stays stable under normal module data.
- Global search shows fast-tier results before deferred modules finish; snapshots do not flicker faster than one frame (~16 ms).
- Repeat identical global queries paint cached results immediately, then refresh in the background without blocking keystrokes.
- Esc from module detail returns to home in one frame when the home snapshot cache is warm (no double Open Apps rebuild).
- Typing a new query while in module detail cancels detail mode and does not restore the suspended prefix on Esc (detail exit via Esc still restores home).
- Notes detail: deactivate/hide during tree refresh must not apply stale outline data (generation guard).
- Hide during split cross-fade must not let stale animation completion rewrite overlay hit-testing.
- Carbon hotkey when panel hidden shows once; visible ⌘Space hides via panel path only (no double toggle).
- Permission banner follows live `searchBar.stringValue`, not a stale normalized snapshot.
- `cmd doctor` only — bare global `doctor` must not run AX/EventKit doctor checks.
- `cmd doctor` reports hotkey registration, corrupt config files, and latency p95 budget.
- Settings → Notes / Projects: choose scan roots; fresh install `n` / `proj` show onboarding rows.
- Switch Space with panel hidden: global hotkey still works after re-register.
- Wake from sleep: Open Apps refreshes within ~5 s.
- Query paste over 8192 chars truncates without crash.
- After upgrade, legacy all-modules-enabled installs migrate to MVP defaults (schema v2); pinned expert modules stay enabled.
- Background cache updates do not repaint visible home.
- Long text and large stores do not resize rows or drift layout.
- Detail open: pooled module detail reopens without rebuilding the view (`detail.viewMade` stays flat on second open).
- Back from detail: home paints cached Open Apps immediately; background revalidate does not flash empty.
- Workbench `proj` preview commands do not stall on selection fetch unless attach/capture is involved.
- Panel hide with Open Apps hidden does not trigger extra Open Apps refresh work.
- Detail open p95 budget: under 50 ms feel; back-to-home p95 budget: under 35 ms feel (manual stopwatch or perf strip).

Permissions:

- Empty home and ordinary app search show no AX banner when Accessibility is denied.
- AX banner appears on Snippets/Menu Bar/Window Layouts targeted surfaces, their detail views, or after Open Apps window controls are used.
- Browser Tabs Automation denial is actionable and not raw AppleScript noise.
- Todo/EventKit denial is actionable.

## Module Manual Smoke

Extended/non-MVP QA for every module in `docs/MODULES.md`. These checks are valuable for release readiness, but parked/default-off modules and Core P1/conditional modules are not P0 gates unless they regress the default P0 path.

- Bare command matches documented behavior.
- Prefix search returns expected rows.
- Global search participation matches the module table.
- Return primary action is correct.
- Tab action panel shows secondary actions without accidental execution.
- Command+Return first secondary action is correct.
- Detail opens, closes, and restores the correct home/query state.
- Empty, error, permission-missing, and dependency-missing states are clear.
- Long text/multiline/large-data cases do not clip or overflow.
- Persistent data survives app restart.
- Destructive actions confirm or show a recovery/status path.

High-value module checks:

- Clipboard: capture, search, copy, pin, delete, clear recent/today, privacy skip.
- Snippets: create, search, Return copy, secondary paste, exact trigger expansion.
- Secrets: locked/unlocked, copy clear timer, value never appears in global search.
- Notes: root config, Tree/Map, create note/folder, open external editor, doctor.
- Todo: create reminder, complete/uncomplete, permission state.
- Wordbook: review, grade, manage, delete confirmation.
- Records: capture/search/edit/delete/export.
- Quicklinks: exact trigger launch, manager add/edit/delete confirmation.
- Projects: current project, links, activity, cross-module draft actions.
- Menu Bar: active app cache, denied AX, stale cache.
- Kill Process: cold refreshing row, search, normal quit, guarded force kill.
- Browser Tabs: default-off note, Automation denied, stale/empty cache refresh.

## Recorded Review

Use a recorded review when doing broad UX work or release readiness.

Recommended run:

```bash
./scripts/run_recorded_review.sh
```

Record:

- Date, build command, git SHA.
- macOS version and display count.
- Input method.
- Accessibility and Automation state.
- Whether `prep_smoke_env.sh` was used.
- Screenshots or video path.

Finding format:

```text
ID:
Severity: P0/P1/P2
Area:
Symptom:
Steps:
Expected:
Actual:
Evidence:
Suggested fix:
```

Severity:

- P0: data loss, app unusable, hotkey/panel broken, security/privacy breach.
- P1: common workflow broken, serious latency/stability issue, confusing failure/no feedback.
- P2: polish, documentation drift, uncommon but real edge case.

## Release

Prerequisites:

- macOS 14+ build host with Xcode command-line tools.
- Developer ID Application certificate for distribution builds.
- Notary credentials via `notarytool` profile or Apple ID/app-specific password.

Useful variables:

| Variable | Purpose |
| --- | --- |
| `LUMA_CODESIGN_IDENTITY` | Explicit signing identity |
| `NOTARY_PROFILE` | Keychain profile for notarization |
| `APPLE_ID` / `APPLE_TEAM_ID` / `APPLE_APP_PASSWORD` | Notary credentials alternative |
| `LUMA_SKIP_NOTARIZATION=1` | Local signed-only DMG |

Build release DMG:

```bash
./scripts/release/build_dmg.sh
```

Signed-only local DMG:

```bash
LUMA_SKIP_NOTARIZATION=1 ./scripts/release/build_dmg.sh
```

Release checklist:

- `swift test` passes.
- **P0 MVP Smoke Gate** (`§ P0 MVP Smoke Gate` above) passes on signed app — run `./scripts/run_p0_smokes.sh` after `./scripts/build_app.sh --no-restart` (not SwiftPM-only).
- `./scripts/build_app.sh --no-restart` passes.
- Release DMG builds and verifies.
- Fresh-machine launch passes Gatekeeper.
- Hotkey and P0 permissions are manually checked. Browser Tabs Automation, Todo/EventKit, and AX-dependent parked/Core P1 surfaces are extended/non-MVP checks and do not gate Phase 9 P0 unless they regress the default P0 path.
- VoiceOver spot check covers search, list rows, detail exit, and Settings.
- Update release notes or tag notes with known limitations.

## Historical QA

Old timestamped QA reports and audit logs are historical evidence, not spec. When they disagree with current behavior, trust:

1. Code and tests.
2. `docs/ENGINEERING.md`.
3. `docs/MODULES.md`.
4. `docs/DECISIONS.md`.
5. This QA file.
