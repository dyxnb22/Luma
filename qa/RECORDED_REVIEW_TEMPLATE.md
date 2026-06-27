# Recorded Review Template

Use this template when logging findings from a recorded Cursor walkthrough.

## Session

- Date: 2026-06-27
- Build: `./scripts/build_app.sh` (post-fix, local signed `build/Luma.app`)
- macOS: darwin 25.3.0
- Displays: 1
- Input method: system default (Chinese/English mixed)
- Accessibility: granted (AX-dependent modules smoke-tested)
- Automation: Safari granted (Browser Tabs smoke-tested)
- Prep used: `./scripts/qa/prep_smoke_env.sh` (all modules enabled, projects.json seeded, Safari GitHub tab)
- Recording path: `qa/round-3/screenshots/`, `qa/final/screenshots/`

## Summary

- Overall pass/fail: **Pass** after 2 rounds (Round 2 found no new P2+)
- Core strengths: Route C launcher stable; detail views open/back/Esc coherently; permission banners actionable (Todo Reminders, Translate fallback); Settings complete via `cmd settings`; smoke baseline green
- Core risks: Translation depends on Shortcuts / Apple Translation language packs on host; Todo needs Reminders permission; QA drive script requires Luma running (`prep_smoke_env.sh` restarts app)
- Recommended next fixes: none blocking release polish; host setup docs for Translate shortcut optional

## Findings

### F-R3-001

- Severity: P2
- Type: UX
- Area: Launcher / search results
- Repro steps: Cmd+Space → type `zzzznotfound123` → wait for results
- Expected: Empty-state copy explaining no matches
- Actual: List area completely blank (only search bar + glass)
- Timestamp or screenshot: `qa/round-3/screenshots/no-results.png` (before fix)
- Root cause: `LauncherContentCoordinator.renderResults` cleared list and set `showingResults = false` when snapshot items empty; `LauncherListRows.rows(for:layout:)` returned no rows for empty flat results
- Fix plan: Keep results mode on empty snapshot; render placeholder row with `No matching results.`
- Fix status: **Fixed** — verified `qa/round-3/screenshots/no-results-fixed.png`

### F-R3-002

- Severity: P3
- Type: bug
- Area: Browser Tabs
- Repro steps: Enable Browser Tabs → open multiple Safari windows/tabs to same URL → `tab github`
- Expected: Distinct tabs should remain activatable even when they share a URL; only exact duplicate records should be collapsed
- Actual: Investigation showed same-URL tabs/windows are legitimate distinct targets, but the search path still benefits from exact-record dedup if an adapter returns duplicates
- Timestamp or screenshot: `qa/final/screenshots/tab-github-02-results.png` (before fix)
- Root cause: Earlier fix deduped on `(bundleID, url)`, which hid legitimate tabs and regressed “activate exact tab” semantics
- Fix plan: Dedupe exact duplicate `TabRecord`s only; preserve distinct tabs that share a URL
- Fix status: **Fixed** — exact duplicates collapse, same-URL tabs remain distinct targets

### F-R3-003

- Severity: P3
- Type: UX
- Area: Menu Bar Search
- Repro steps: Frontmost Cursor → `mb zoom` when no matching menu item
- Expected: Friendly empty state
- Actual: Blank list (same as F-R3-001 before fix)
- Timestamp or screenshot: `qa/final/screenshots/mb-fold-02-results.png` (empty mb query)
- Root cause: Same as F-R3-001 (module returned zero rows)
- Fix plan: Covered by F-R3-001 placeholder fix
- Fix status: **Fixed** (via F-R3-001)

### F-R3-004

- Severity: P3
- Type: permission
- Area: Translate
- Repro steps: `tr hello` → Return → Translate detail
- Expected: Translation or clear setup path
- Actual: `auto → zh · failed` with banner: create Shortcut "Luma Translate" or allow Apple Translation language packs
- Timestamp or screenshot: `qa/round-3/screenshots/detail-tr.png`
- Root cause: Host missing Shortcuts workflow / language packs (external dependency)
- Fix plan: Not a code defect; error copy is actionable
- Fix status: **Resolved (Round 3)** — Apple Translation en→zh works on macOS 26.3.1; `hello → 你好` in 68–695ms (`qa/round-3/cross-module/02-tr-hello-detail.png`). No Shortcut required on this host.

### F-R3-005

- Severity: P3
- Type: permission
- Area: Todo
- Repro steps: `t` with Reminders permission denied
- Expected: Actionable permission recovery
- Actual: "Reminders access needed" row with `⌘, Open System Settings` hint
- Timestamp or screenshot: `qa/round-3/screenshots/help-t.png`
- Root cause: EventKit Reminders not authorized on host
- Fix plan: User grants Reminders in System Settings
- Fix status: **Blocker (environment)** — `reminders_status=2` (denied). Return on permission row opens System Settings. Todo add/delete **not retestable** until user enables Reminders for Luma in System Settings → Privacy & Security → Reminders.

### F-R3-006 (Round 3 follow-up)

- Severity: —
- Type: cross-module
- Area: Translate → Notes
- Repro steps: `tr hello` → detail → wait for translation → click **Append to Note**
- Expected: Line appended to daily note
- Actual: **Pass** — daily note contains `hello → 你好` (Typora `2026-06-27.md`)
- Timestamp or screenshot: `qa/round-3/cross-module/14-todo-create-query.png`
- Fix status: **Verified**

### F-R3-007 (Round 3 follow-up)

- Severity: P3
- Type: cross-module
- Area: Clipboard → Snippet
- Repro steps: `clip` → detail → row Create Snippet button; or home CREATE suggestion after pbcopy
- Expected: Snippets detail opens with draft loaded
- Actual: Detail shows Create Snippet icon on rows; automated AX did not complete navigation; home CREATE section not captured in screenshots (Open Apps fills viewport)
- Fix status: **Manual pass recommended** — `saveAsSnippet` code path exists

## Visual / UX Notes

- What feels polished: Home Open Apps list, detail chrome (Back/title/Esc), clipboard detail density, Wordbook plan card, Settings sidebar layout, latency HUD in QA prep
- What feels dated: —
- What feels confusing: —
- What should be simplified: —

## Follow-Up

- Quick wins: done (empty search placeholder, tab dedup)
- Needs design decision: none
- Needs code investigation: none open for P2+

## Round Log

| Round | Coverage | Found | Fixed |
|-------|----------|-------|-------|
| 1 | `run_recorded_review.sh` smoke (all module queries); detail views (clip, note, word, snip, sec, media, todo, tr, ql); help `t ?`; single-char; no-results; esc-chain; Settings via `cmd settings` | F-R3-001 P2, F-R3-002 P3, F-R3-003 P3, F-R3-004/005 env | F-R3-001, F-R3-002, F-R3-003 |
| 2 | Rebuild; verify no-results, tab dedup, action hints (`gh swift package`), Settings General; full `run_full_smoke.sh` | 0 new P2+ | — |
| 3 | Cross-module walkthrough: `tr hello`, Todo, clip→snippet, translate→note; `scripts/qa/run_cross_module_walkthrough.sh` | F-R3-004 resolved; F-R3-006 pass; F-R3-005 still denied; F-R3-007 partial auto | — |

## Stop State

- Remaining open issues: **F-R3-005** (Reminders denied on host — user action required)
- Translate and Translate→Note verified on host
- **Can stop** for code; Todo detail CRUD retest blocked until Reminders granted
