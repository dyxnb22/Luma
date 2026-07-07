# Phase 9.8 — P0 MVP Smoke Gate Report

**Date:** 2026-07-07 09:50 HKT  
**Branch:** `main` @ `889ebd35` (clean working tree)  
**Verdict:** **Go for P0 MVP Exit** ✅

---

## Executive Summary

Phase 9.1–9.7 P0 recovery slices were run as a single signed-app gate. Build, core tests, env-gated production smokes, diagnostics export, and crash baseline all passed. No new `Luma-*.ips` were produced during this gate. Luma has recovered from “basic paths broken / unverified” to **MVP-usable on the defined P0 main path**.

Residual risk is concentrated in **manual hotkey/Esc soak**, **long-term 50/80 ms latency ceiling**, and **P1/P2 polish** — not in blocking P0 functionality.

---

## 1. Final Baseline

| Check | Result |
|-------|--------|
| `git status` | Clean (`main`, up to date with `origin/main`) |
| `swift build` | ✅ Pass |
| `./scripts/build_app.sh --no-restart` | ✅ Signed `build/Luma.app` |
| Signed app starts | ✅ All smoke runs: `pgrep -x Luma` confirmed real `Luma` process (not Cursor Helper) |
| `.ips` before gate | **3** |
| `.ips` after all smokes | **3** (unchanged) |

### Historical `.ips` (pre-gate, not introduced by Phase 9 gate)

| File | Time (local) | Notes |
|------|--------------|-------|
| `Luma-2026-07-06-115651.ips` | 11:56 | `NotesMindMapView.isFlipped` — fixed Phase 9.1 |
| `Luma-2026-07-06-165734.ips` | 16:57 | AppKit executor |
| `Luma-2026-07-06-184548.ips` | 18:45 | Post Phase 9.1 |

### Latency baseline (`~/Library/Logs/Luma/latency-report.json`)

| Metric | Value | P0 ceiling | Status |
|--------|-------|------------|--------|
| `hotkeyP95Milliseconds` | **28.08 ms** | ≤ 1000 ms (emergency) | ✅ Pass |
| `keystrokeP95Milliseconds` | **19.98 ms** | ≤ 120 ms (engineering) | ✅ Pass |
| `hotkeySampleCount` | 33 | — | Includes one legacy ~108 ms sample in array; current p95 is healthy |
| `generatedAt` | 2026-07-06T14:46:57Z | — | Not refreshed during this gate session |

**Long-term risk (P1/P3):** Engineering target remains 50 ms / ceiling 80 ms for hotkey→visible. Current p95 (~28 ms) is within budget; stale historical 8.3 s p95 (USABILITY_TRIAGE S-002) is not reproduced in latest telemetry.

---

## 2. Core Tests

| Filter | Result | Count |
|--------|--------|-------|
| `AppsModuleTests` | ✅ Pass | 4 |
| `Clipboard` | ✅ Pass | 56 |
| `Notes` | ✅ Pass | 49 |
| `Settings` | ✅ Pass | 2 |
| `Config` | ✅ Pass | 9 |
| `Persistence` | ✅ Pass | 3 |
| `DiagnosticsExport` | ✅ Pass | 5 |
| `LauncherActionDispatch` | ✅ Pass | 13 |

### Non-P0 / backlog (passed this run, not gate blockers)

| Test | Result | Note |
|------|--------|------|
| `appsModuleTopTargetedQueryStaysUnderBudget` | ✅ Pass (p95 within budget this run) | Previously flagged P1/P3 perf jitter |

No Phase 9 `*ProductionSmoke*` unit tests exist (smokes are signed-app env hooks only).

---

## 3. Signed App QA Smoke (env-gated)

Each smoke: kill prior `Luma` → env launch → wait 12s → verify JSON → kill.  
**Normal launch** after gate did **not** update smoke JSON mtimes.

| Env | Output JSON | Key results |
|-----|-------------|-------------|
| `LUMA_QA_EXPORT=1` | `~/Library/Logs/Luma/diagnostics.json` | Export succeeded; payload populated |
| `LUMA_QA_APPS=1` | `apps-smoke.json` | `launchResult: success`, targeted/global Safari rows |
| `LUMA_QA_CLIPBOARD=1` | `clipboard-smoke.json` | `copySucceeded: true`, `detailListMs ~35`, `bareClipOpensDetail: true` |
| `LUMA_QA_NOTES=1` | `notes-smoke.json` | create/search/mindMap/containment/config restore all ✅ |
| `LUMA_QA_SETTINGS=1` | `settings-smoke.json` | persistence round-trip, single settings window, hotkey honesty |

### Config restore after smokes

- `notes.json` root restored to `/Users/diaoyuxuan/.qa-luma-notes` (`diskRootMatchesBackup: true` in notes smoke)
- Settings smoke: `latencyHUDRestored: true`, `clipboardMaxEntriesRestored: true`

---

## 4. P0 Main Path Coverage

| # | Path | Gate evidence | Status |
|---|------|---------------|--------|
| 1 | Signed app starts / stays running | All 5 smoke runs + export | ✅ |
| 2 | No new `.ips` | 3 → 3 | ✅ |
| 3 | Hotkey show/hide | Latency p95 28 ms; registration `hotkeyRegistered: true` in diagnostics; **manual ×20 toggle not re-run this session** | ⚠️ Automated pass; manual soak deferred |
| 4 | Menu bar Show fallback | `MenuBarController` → `windowController.show()` (Phase 9.2); code path verified, not UI-clicked this session | ⚠️ Code + prior phase; manual click deferred |
| 5 | Launcher input | Keystroke p95 20 ms; LauncherActionDispatch tests | ✅ |
| 6 | Apps search/open | `apps-smoke.json` Safari launch success | ✅ |
| 7 | Clipboard detail/search/copy | `clipboard-smoke.json` | ✅ |
| 8 | Notes detail/onboarding/create/search/open | `notes-smoke.json` | ✅ |
| 9 | Settings open/save | `settings-smoke.json` | ✅ |
| 10 | Doctor/Export outside Commands | Menu bar entries + `LUMA_QA_EXPORT` + `commandsModuleDefaultOff: true` | ✅ |

### Manual / UI paths (not re-clicked in 9.8; covered by smoke + Phase 9 slices)

- Cmd+Space show/hide, Esc/hide/reshow input, menu bar Show/Settings/Doctor clicks — recommend `docs/QA.md` checklist before release tag.

---

## 5. Diagnostics Payload Summary

**Path:** `~/Library/Logs/Luma/diagnostics.json` (from `LUMA_QA_EXPORT=1`)

| Field | Present | Value / note |
|-------|---------|--------------|
| `platform` | ✅ | OS version, screen count, presentation screen |
| `modules` | ✅ | `enabledCount: 13`, `totalCount: 16` |
| `modules.enabledModuleIDs` | ✅ | 13 IDs — reflects real enabled set (no P0 injection) |
| `modules.mvpCoreModuleStatus` | ✅ | Apps / Clipboard / Notes each `enabled: true` |
| `permissions` | ✅ | `hotkeyRegistered: true`, `accessibilityTrusted: true` |
| `breadcrumbs` | ✅ | `[]` (empty this session) |
| `recentErrors` | ✅ | `[]` |
| `latencyP95Milliseconds` | ✅ | `0` (no samples in current session export) |
| `crashLogPath` | ✅ | `~/Library/Application Support/Luma/crash-log.txt` |
| `crashLogWriteStatus` | ✅ | `available` |
| `corruptConfigFiles` | ✅ | `[]` |

**Commands default-off:** `commandsModuleDefaultOff: true` in settings smoke; menu bar Doctor/Export independent of Commands module.

---

## 6. Crash Regression

| Check | Result |
|-------|--------|
| `.ips` before | 3 |
| `.ips` after all smokes | 3 |
| Notes MindMap smoke | `mindMapReloadSucceeded: true`, no new crash |
| Clipboard AppKit warn sites | Not exercised to crash; 13 warn-only sites remain (P2 cleanup) |

---

## 7. Phase 9 Code Delivered (committed on `main`)

| Area | Key files |
|------|-----------|
| 9.1 AppKit crash | `LauncherHomeGuidePane.swift`, `NotesMindMapView.swift` |
| 9.2 Hotkey/latency | `HomeLatencyTracker.swift`, `LauncherWindowController.swift`, `LauncherRootController.swift` |
| 9.3 Diagnostics recovery | `MenuBarController.swift`, `RecoveryDiagnosticsCollector.swift`, `RecoveryDiagnosticsPresenter.swift`, `AppHostService.swift` |
| 9.4 Apps smoke | `AppsProductionSmoke.swift` |
| 9.5 Clipboard | `ClipboardProductionSmoke.swift`, `ClipboardDetailView.swift` (copy/paste feedback) |
| 9.6 Notes | `NotesProductionSmoke.swift` |
| 9.7 Settings | `SettingsProductionSmoke.swift`, `SettingsSwiftUIView.swift` (hotkey honesty), `JSONConfigPersistence.swift`, `RecoveryDiagnosticsCollector.swift` |

**Not changed in gate:** parked modules, `ConfigurationStore` rewrite, LauncherRootController refactor, module governance, docs cleanup.

---

## 8. P0 MVP Exit Judgment

### **Go** ✅

All blocking P0 criteria met:

- Signed app builds and runs
- No new crashes during gate
- Apps / Clipboard / Notes / Settings / Diagnostics P0 paths verified via production smokes
- Diagnostics export semantically correct (`enabledModuleIDs`, `mvpCoreModuleStatus`, crash-log path)
- Hotkey latency within 1 s emergency ceiling
- Recovery entries reachable without Commands module

### No-Go blockers

*None.*

---

## 9. P1 / P2 / P3 Backlog (do not fix in P0 exit)

### P1

| Item | Source |
|------|--------|
| Manual QA soak: Cmd+Space ×20, rapid toggle ×50, Esc from home/detail/action panel | `docs/QA.md` |
| Menu bar Show bypasses Carbon debounce/guard (S-003) | USABILITY_TRIAGE |
| `appsModuleTopTargetedQueryStaysUnderBudget` perf jitter | Phase 9.4 |
| Clipboard 38 MB history + `.corrupt-*.bak` housekeeping | USABILITY_TRIAGE S-016 |
| Launcher perform failure visible toast (e.g. `n new` Return) | Phase 9.6 |
| UserDefaults corruption/fallback visibility | Phase 9.7 |
| `launcherFlowHarnessReplaysQuery` intermittent failure | CURRENT_STATE |

### P2

| Item | Source |
|------|--------|
| `JSONConfigPersistence` unreadable vs `wasCorrupt` / `loadWasCorrupt` semantic split | Phase 9.7 review |
| `SettingsProductionSmoke` `HotkeyConfig.resetToDefault()` legacy key cleanup | Phase 9.7 review |
| ClipboardDetailView 13 AppKit executor warn-only sites | Phase 9.5 |
| Notes detail cold-index warming UI | Phase 9.6 |
| Settings Notes root picker snapshot refresh without reopen | Phase 9.7 |
| Full `LoadResult.loadIssue` enum | Phase 9.7 |

### P3

| Item | Source |
|------|--------|
| Long-term hotkey 50/80 ms ceiling vs current ~28 ms p95 | ENGINEERING.md |
| Parked module smoke (Media, Wordbook, Commands scripts, etc.) | MVP_SCOPE |
| Core P1 modules (Snippets, Quicklinks, Translate, Todo) | MVP_SCOPE |
| Docs/PERMISSIONS default governance | PHASE8 |

---

## 10. Commands Run (repro)

```bash
swift build
./scripts/build_app.sh --no-restart

# Tests
swift test --filter AppsModuleTests
swift test --filter Clipboard
swift test --filter Notes
swift test --filter Settings
swift test --filter Config
swift test --filter Persistence
swift test --filter DiagnosticsExport
swift test --filter LauncherActionDispatch

# Smokes (one at a time)
pkill -x Luma; LUMA_QA_EXPORT=1 build/Luma.app/Contents/MacOS/Luma
pkill -x Luma; LUMA_QA_APPS=1 build/Luma.app/Contents/MacOS/Luma
pkill -x Luma; LUMA_QA_CLIPBOARD=1 build/Luma.app/Contents/MacOS/Luma
pkill -x Luma; LUMA_QA_NOTES=1 build/Luma.app/Contents/MacOS/Luma
pkill -x Luma; LUMA_QA_SETTINGS=1 build/Luma.app/Contents/MacOS/Luma

# Verify
ls ~/Library/Logs/Luma/*smoke*.json ~/Library/Logs/Luma/diagnostics.json
ls ~/Library/Logs/DiagnosticReports/Luma*.ips | wc -l
```

---

*Gate executed without `git add` / `commit`. No code changes required during Phase 9.8.*
