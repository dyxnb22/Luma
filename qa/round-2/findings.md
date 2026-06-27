# Round 2 Findings

## Fixed (verified screenshots in `screenshots/r2-final-*`)

### F-2-01 Quicklinks global search (`gh swift package`) — was F-1-01
- **Fix:** `Ranker` now ranks targeted payloads and per-token matches instead of requiring the full query to fuzzy-match result titles.
- **Screenshot:** `r2-final-gh.png` — GitHub Search row under QUICKLINKS.

### F-2-02 App search (`Safari`) — was F-1-01
- **Fix:** `AppScanner` resolves symlinked `.app` bundles and scans `/System/Cryptexes/App/System/Applications`.
- **Screenshot:** `r2-final-Safari.png` — Safari row under APPS.

### F-2-03 Session restore / QA text pollution — was F-1-04
- **Fix:** `drive.sh` clears persisted launcher query; `lumaQASkipRestore` defaults flag; `LauncherRootController` skips restore during QA.
- **Screenshot:** clean field on `open` before each case.

### F-2-04 Query sync / automation input — was F-1-01 hypothesis
- **Fix:** `LumaSearchBar.commitEditingIfNeeded`, spurious-empty guard in `syncQueryIfNeeded`, `controlTextDidEndEditing`, `paste_query.swift` keycode + AX commit.
- **Note:** AppleScript `set value` does not work on Luma’s search field; keycodes required.

## Open

### F-2-05 Kill Process (`kill preview`) — was F-1-02
- **Status:** No rows when Preview is not running (`pgrep Preview` empty on this machine).
- **Severity:** P3 (environmental) unless Preview is running and still empty.
- **Screenshot:** `r2-final-kill-preview.png`

### F-2-06 Menu Bar Search (`mb fold`) — was F-1-03
- **Status:** Command hints render; menu rows still empty after `LauncherMenuTarget` context fix.
- **Hypothesis:** `MenuBarTreeService.walk` 200ms budget + async cache warm; Cursor menu tree may not populate in time during automated runs.
- **Severity:** P2
- **Screenshot:** `r2-final-mb-fold-v2.png`
- **Manual check:** Activate target app → hotkey → `mb fold` with Accessibility enabled.

## Harness changes
- `scripts/qa/drive.sh` — persisted-query clear, `paste_query.swift` integration, longer open settle time.
- `scripts/qa/paste_query.swift` — ABC keyboard, AX clear/commit, keycode typing.

## Tests added
- `RankerTests` — payload + quicklink token ranking
- `GlobalSearchDispatchTests` — Safari scan/dispatch on this machine
- Total: **362 tests green**
