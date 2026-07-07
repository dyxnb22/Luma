# P0 Exit Summary

**Date:** 2026-07-07  
**Baseline commit:** `889ebd35` — *Add QA smoke hooks and config corruption tracking*  
**Branch:** `main` (up to date with `origin/main`)  
**P0 Exit verdict:** **Go** ✅  
**Phase 9 status:** **Closed** — P0 MVP recovery complete; P1 may begin.

---

## Baseline integrity (Phase 10 check)

| Check | Result |
|-------|--------|
| `git log -1 --oneline` | `889ebd35 Add QA smoke hooks and config corruption tracking` |
| Swift source modified? | **No** uncommitted source changes |
| Untracked docs | `PHASE9_MVP_SMOKE_REPORT.md` (Phase 9.8 artifact, not yet committed) |
| Phase 9.8 report hash | Matches current HEAD (`889ebd35`) |

Phase 9.8 reported a clean tree at gate time; the only delta since then is the untracked gate report file above.

---

## Phase 9 slice results (9.1 → 9.8)

| Slice | Focus | Result | Key deliverable |
|-------|--------|--------|-----------------|
| **9.1** | Runtime / AppKit crash baseline | ✅ Pass | `LauncherHomeGuidePane` `nonisolated` delegates; `NotesMindMapView.isFlipped` boundary |
| **9.2** | Hotkey / launcher entry | ✅ Pass | Hotkey p95 measurement fix; latency HUD wiring |
| **9.3** | Diagnostics / Doctor / export recovery | ✅ Pass | Menu bar Run Doctor / Export Diagnostics; `RecoveryDiagnosticsCollector` |
| **9.3.1** | Diagnostics payload semantics | ✅ Pass | `enabledModuleIDs` / `mvpCoreModuleStatus` separation |
| **9.4** | Apps search / open | ✅ Pass | `LUMA_QA_APPS=1` → `apps-smoke.json` |
| **9.5** | Clipboard search / copy | ✅ Pass | `ClipboardProductionSmoke`; row/menu copy/paste feedback |
| **9.6** | Notes open / create | ✅ Pass | `LUMA_QA_NOTES=1` → `notes-smoke.json` |
| **9.7** | Settings open / save | ✅ Pass | `LUMA_QA_SETTINGS=1`; hotkey UI honesty; config corruption visibility |
| **9.8** | P0 MVP smoke gate | ✅ **Go** | Full gate; see `PHASE9_MVP_SMOKE_REPORT.md` |

---

## P0 main path acceptance

| Path | Status | Evidence |
|------|--------|----------|
| Signed app starts / stays running | ✅ | All env-gated smoke runs; `pgrep -x Luma` confirmed real process |
| No new `.ips` | ✅ | 3 before gate → 3 after (2026-07-06 historical only) |
| Hotkey show/hide | ✅* | `hotkeyP95Milliseconds` **28 ms** (≤ 1 s P0 ceiling); `hotkeyRegistered: true` in diagnostics |
| Menu bar Show fallback | ✅* | Code path verified Phase 9.2; manual click soak deferred to P1 |
| Launcher input | ✅ | Keystroke p95 **20 ms**; `LauncherActionDispatch` tests pass |
| Apps search/open | ✅ | `apps-smoke.json`: `launchResult: success` |
| Clipboard search/copy | ✅ | `clipboard-smoke.json`: `copySucceeded: true`, `detailListMs ~35` |
| Notes open/create/search | ✅ | `notes-smoke.json`: create, search, mindMap, containment, config restore |
| Settings open/save | ✅ | `settings-smoke.json`: persistence round-trip, single window |
| Diagnostics / Doctor / export | ✅ | Menu bar entries; `LUMA_QA_EXPORT=1` → `diagnostics.json`; Commands default-off does not block |

\*Automated + code-path verified; full manual soak (×20 hotkey, ×50 rapid toggle) remains **P1** release checklist.

---

## Core P1 / parked modules — not in P0 gate

The following were **explicitly excluded** from Phase 9 P0 acceptance:

- **Parked / deferred:** Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench/Capture, Commands user scripts
- **Core P1 / conditional:** Snippets, Quicklinks, Translate, Todo (unless they regress the default P0 path)

P0 gate covers only: **Launch → Hotkey/Menu → Launcher input → Apps → Clipboard → Notes → Settings → Diagnostics recovery.**

---

## Test commands and results (Phase 9.8)

```bash
swift build                                    # ✅
./scripts/build_app.sh --no-restart            # ✅

swift test --filter AppsModuleTests            # ✅ 4
swift test --filter Clipboard                  # ✅ 56
swift test --filter Notes                      # ✅ 49
swift test --filter Settings                   # ✅ 2
swift test --filter Config                     # ✅ 9
swift test --filter Persistence                # ✅ 3
swift test --filter DiagnosticsExport          # ✅ 5
swift test --filter LauncherActionDispatch     # ✅ 13
```

No Phase 9 `*ProductionSmoke*` SwiftPM unit tests exist; verification is signed-app env hooks only.

---

## Signed app smoke artifacts

All under `~/Library/Logs/Luma/`:

| Env var | Artifact | Phase 9.8 key fields |
|---------|----------|----------------------|
| `LUMA_QA_EXPORT=1` | `diagnostics.json` | platform, modules, permissions, crashLogPath, mvpCoreModuleStatus |
| `LUMA_QA_APPS=1` | `apps-smoke.json` | `launchResult: success` |
| `LUMA_QA_CLIPBOARD=1` | `clipboard-smoke.json` | `copySucceeded: true` |
| `LUMA_QA_NOTES=1` | `notes-smoke.json` | `createdFileExists`, `configRestored`, `diskRootMatchesBackup` |
| `LUMA_QA_SETTINGS=1` | `settings-smoke.json` | `latencyHUDPersisted`, `clipboardMaxEntriesRestored` |

**Restore policy:** Notes and Settings smokes backup/restore user config. After each smoke, confirm `notes.json` root and Settings toggles match pre-smoke values. Mid-smoke kill can leave transient state — re-run restore path or manual check.

**Normal launch** must not set `LUMA_QA_*` env vars; smokes do not run on ordinary startup.

---

## Latency summary

Source: `~/Library/Logs/Luma/latency-report.json` (2026-07-06T14:46:57Z)

| Metric | Value | P0 emergency | Engineering target |
|--------|-------|--------------|-------------------|
| Hotkey p95 | **28.08 ms** | ≤ 1000 ms ✅ | 50 ms / 80 ms ceiling (P3 residual) |
| Keystroke p95 | **19.98 ms** | — | ≤ 120 ms ✅ |

Historical ~8.3 s hotkey p95 (USABILITY_TRIAGE S-002) not reproduced in latest telemetry.

---

## Crash / `.ips` summary

| | Count |
|---|-------|
| Before Phase 9.8 gate | 3 |
| After all smokes | 3 |

Historical crashes (2026-07-06):

- `Luma-2026-07-06-115651.ips` — `NotesMindMapView.isFlipped` (fixed 9.1)
- `Luma-2026-07-06-165734.ips` — AppKit executor
- `Luma-2026-07-06-184548.ips` — post-9.1

`crash-log.txt`: `~/Library/Application Support/Luma/crash-log.txt` (`crashLogWriteStatus: available` in export).

---

## Regression protection (mandatory for all P1/P2/P3 PRs)

1. **Before merge:** Run the full **P0 MVP Smoke Gate** (`docs/QA.md` § P0 MVP Smoke Gate). SwiftPM green alone is **not** sufficient (C-TEST-004, S-026).

2. **New `.ips`:** Stop feature work → P0 runtime triage (Phase 9.1 class fixes only until clear).

3. **Hotkey p95 > 1 s** on signed app: Stop → return to **P0.2** hotkey/latency slice.

4. **Doctor / export unreachable** from menu bar (Commands default-off): Stop → return to **P0.8** diagnostics recovery.

5. **Any P0 module path fails** (Apps, Clipboard, Notes, Settings): Do not proceed with P1 Launcher refactor until smoke gate passes again.

6. **Do not** pull parked modules or Core P1 surfaces into P1 PRs unless explicitly scoped and gated separately.

---

## Backlog (P1 / P2 / P3) — not in P0 exit scope

### P1 — start after P0 gate; order per `REFACTOR_PLAN.md` §6

1. `LauncherRootController` boundary (P1.1)
2. Launcher session state owner (P1.2) — includes menu bar Show / debounce semantics (S-003)
3. Detail lifecycle (P1.3)
4. Task / MainActor boundary cleanup (P1.4) — Clipboard warn sites, detail views
5. Cache refresh vs UI repaint separation (P1.5)

Additional P1 items:

- Manual soak: Cmd+Space ×20, rapid toggle ×50, Esc from home/detail/action panel
- `launcherFlowHarnessReplaysQuery` production divergence (CURRENT_STATE intermittent failure)
- 38 MB Clipboard history performance / `.corrupt-*.bak` housekeeping (S-016)
- Launcher perform failure visible toast (`n new` Return path)
- UserDefaults corruption/fallback visibility

### P2

- `JSONConfigPersistence`: `wasCorrupt` vs unreadable semantic split (`loadIssue`)
- `SettingsProductionSmoke` `HotkeyConfig.resetToDefault()` legacy key behavior
- ClipboardDetailView 13 AppKit executor warn-only sites
- Notes cold-index warming UI
- Settings Notes root picker snapshot refresh
- Module diagnostic consistency (P2.2)
- Module lifecycle contract unification for MVP modules (P2.1)

### P3

- Long-term hotkey 50/80 ms budget enforcement vs current ~28 ms p95
- `docs/PERMISSIONS.md` stale default hygiene
- Parked / Core P1 module re-entry criteria and smoke
- Real smoke automation hardening (scripted gate replacing manual env launches)
- `LauncherFlowHarness` production parity (P3.4)

---

## Phase closure

| Question | Answer |
|----------|--------|
| Can Phase 9 be formally closed? | **Yes** |
| P0 MVP usable? | **Yes** on defined main path |
| Next recommended phase | **Phase 11 — P1 Launcher complexity reduction** (P1.1 → P1.5 per `REFACTOR_PLAN.md`) |

---

## References

- `PHASE9_MVP_SMOKE_REPORT.md` — Phase 9.8 gate detail
- `MVP_SCOPE.md` — P0 feature set and acceptance
- `REFACTOR_PLAN.md` — P0 achieved; P1 entry conditions
- `docs/QA.md` — P0 MVP Smoke Gate checklist
