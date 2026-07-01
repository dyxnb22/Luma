# Manual QA Checklist

## Recorded Review Setup

- Build the current app with `./scripts/build_app.sh`.
- If the pass should be deterministic, run `./scripts/qa/prep_smoke_env.sh` first.
- Confirm whether the session is a scripted smoke pass, a freeform product-review pass, or both.
- Start the recording before the first launcher invocation so first-run and permission states are captured.
- Record machine context: macOS version, display count, input method, Accessibility state, Automation state.

## Review Lenses

- **Functional correctness:** feature works end to end and state persists as expected.
- **Usability:** labels, flows, defaults, and recovery paths are understandable without prior project context.
- **Keyboard-first quality:** a user can complete the flow confidently without reaching for the mouse.
- **Visual polish:** spacing, density, hierarchy, alignment, empty states, and feedback feel modern and intentional.
- **Performance feel:** the UI looks stable and immediate, not just technically correct.

## Hotkey

- Works after launch.
- Default chord is Command+Space.
- Toggles panel when panel is visible.
- Still works after sleep/wake.
- Does not collide with Spotlight or input method shortcuts.

## Panel

- Appears on active display.
- Appears over fullscreen apps.
- Appears on all Spaces as expected.
- Query field is focused immediately.
- Escape dismisses.
- Focus loss behavior matches Settings.

## Performance

- Hotkey -> panel p95 <= 50 ms.
- Keystroke -> first result p95 <= 30 ms.
- No visible row jumping while typing.

## Permissions

- Accessibility denied state is clear.
- Accessibility granted state enables window focus.
- Settings links to System Settings when permission is missing.
- Automation-denied state for Browser Tabs is actionable and not raw AppleScript noise.
- Permission banners do not trap keyboard focus or block Esc dismissal unexpectedly.

## Clipboard

- Does not store concealed/transient pasteboard values.
- Skips the entire pasteboard change if any blocked type is present.
- Enforces 500-entry, 7-day, and 100-KB body caps.
- Enforces retention caps.
- Clear history command works.

## Launcher Home And Results (Route C)

### Panel Glass

- Liquid glass background visible.
- Top highlight gradient visible.
- 1 pt inner border visible.
- 20 pt continuous corner radius.
- Panel does not bleed background on light desktops.

### Home List

- Open Apps section appears with 11 pt uppercase header.
- Open Apps rows render in the main unified list, not a permanent sidebar.
- Order changes after switching apps externally.
- Open Apps does not list Luma itself.
- Return or click on an app row activates that app.

### Search Results

- Typing switches from home sections to search results within ~80 ms.
- Clearing text restores the home list.
- Up/Down moves selection without rebuilding rows.
- Selected row shows accent background and Return hint capsule.
- Return runs the selected item and hides the panel.
- Selection sticks to the previously selected item if it remains in the new result set.

### Module Detail

- Running a detail entry swaps the main content area to a `BaseDetailContainer` detail view (16 pt margins).
- Search bar remains visible.
- Top bar shows Back, module title, and close.
- Esc in detail returns to the home list (does not close the panel).
- Esc again on the home list closes the panel.

### Latency HUD

- Settings → General → "Show latency HUD overlay" toggles bottom-right p95 overlay.
- Overlay updates after each search paint when enabled.

### Apps RAM

- `app top` lists running apps by resident memory; Return activates; secondary action quits app.

### Clipboard → Snippet

- Clipboard detail row action saves entry via snippet editor sheet.

## Clipboard Global Search

- Type 3+ characters in global search → clipboard entries matching the query appear inline (up to 3 rows, ranked alongside other modules).
- Queries shorter than 3 characters do not surface clipboard entries.
- `clip` / `cb` prefix still works as before and is not affected.
- Privacy: concealed / transient pasteboard types are never captured and never appear in global search results.

### Settings Activity

- Settings → Activity tab shows 7-day and 30-day sparklines plus per-module counts.

### Module Help

- `?` / `help` on module triggers: `t ?`, `s ?`, `m ?`, `secret ?`, `clip ?`, `tr ?`, `app ?`, and global `?` for commands.

### Hotkey Status

- Menu bar shows ⌘ icon when hotkey is registered.
- Menu bar shows lock icon when secrets vault is locked; open lock when unlocked.
- Menu bar shows triangle icon plus "Disable Spotlight's ⌘+Space" entry when registration fails.

## Snippets

- `s` / `snip` result rows and actions open the relevant snippet flow.
- Add snippet via detail → `s <query>` finds it → Return copies content.
- Tab on a snippet result pastes into front app (requires Accessibility).
- Edit / Delete / Duplicate round-trip across app restart.
- **Trigger expansion**: create a snippet with a trigger (e.g. `;hdr`) → type `;hdr` in the launcher search bar → Return expands and pastes inline → panel dismisses → status bar shows "Snippet expanded".
- Trigger expansion only fires in global search mode (not when already scoped to `s` / `snip`).
- Trigger match is case-insensitive; a query with spaces does not trigger expansion.

## Secrets

- `secret` result rows and actions open the relevant secrets flow.
- Locked state shows Unlock button; unlocked shows CRUD table.
- Add secret persists across restart; value only in Keychain.
- `secret unlock` then `secret <label>` lists secrets in launcher.
- Copy secret → pasteboard cleared after 10 s if not overwritten.
- After 5 min idle, vault auto-locks; menu bar icon updates.
- Settings: auto-clear, re-lock timeout, require-unlock-on-launch.

## Records (`luma.media`)

- `m Oppenheimer movie 9` captures and appears in `m oppen` search.
- `m` shows recent items; `m log` opens detail view.
- Detail view: category tabs, status filter, sort work on empty and populated lists.
- Add / Edit / Delete round-trip across restart.
- Export CSV writes to Downloads and opens in Finder.
- Tab on search result copies `title — rating/10`.

## Quicklinks

- Cmd+Space → `gh swift package` shows GitHub Search with an encoded GitHub URL.
- Return opens the URL in the default browser.
- Tab on the row shows Copy URL and Reveal Quicklinks Config.
- `ql` opens the same-panel Quicklinks manager.
- Add, edit, delete one quicklink and confirm the trigger works after returning to results.
- Test Chinese, emoji, quote characters, duplicate triggers, malformed URL templates, and missing config file.

## Menu Bar Search

- Put Cursor frontmost → Cmd+Space → `mb fold` shows matching menu items with path and shortcut.
- Return on a menu item performs the command without leaving a raw AX error.
- `mb ?` shows help rows.
- Switch frontmost apps (Finder, Safari, Xcode/Cursor if installed) and confirm results follow the active app cache.
- Deny Accessibility permission and verify the permission banner / friendly failure appears.
- Test disabled bundle config, no-window apps, rapid typing, and Escape within 100 ms of typing.

## Kill Process

- Cmd+Space → `kill` lists recent running GUI apps, excluding Luma.
- `kill preview` finds Preview when running and shows bundle ID plus memory.
- Return sends normal Quit and the list refreshes.
- Tab shows Force Kill and Relaunch; Force Kill requires the second modifier.
- Finder/Dock/SystemUIServer rows require confirmation before quit/relaunch.
- Test empty list behavior, app relaunch, fuzzy bundle ID search, and rapid typing.

## Browser Tabs

- Enable Browser Tabs in Settings → Modules.
- Open Safari/Chrome tabs → Cmd+Space → `tab github` shows matching title/URL rows.
- Return activates the browser window and selects the tab.
- `tab ?` shows help rows and notes Automation permission.
- Test Safari closed, Chrome not running, Arc/Edge/Brave in background, duplicate titles, stale cache after tab switch.
- Deny Automation permission and verify the failure is user-facing rather than raw script output.

## Auto Workflow

- Detailed pass: `docs/qa/AUTOWORKFLOW_UI_ACCEPTANCE.md`.
- Preflight: `./scripts/qa/autoworkflow_preflight.sh`.
- Evidence collection: `./scripts/qa/autoworkflow_collect.sh`.
- Enable Auto Workflow in Settings → Modules.
- Settings → Auto Workflow shows source path and `cc-loop` availability accurately.
- Cmd+Space → `aw` opens the same-panel Auto Workflow detail view.
- Empty goal/repo cannot start and shows a user-facing error.
- Valid goal/repo runs doctor → init → detached start and shows task ID, PID, polling status, and log tail.
- Stop terminates the runner without killing unrelated processes; stopped/interrupted tasks expose Resume when `cc-loop` reports it.
- Hide and reopen the launcher; polling is stopped while hidden and resumes from task status when the panel is opened again.
- Test missing source path, missing `cc-loop`, large log tail, and prefixed status/list output.

## Notes v0.1

- Root picker writes `notes.json`.
- Launcher hit: type `tree`, Return opens `Tree.md` in Typora.
- Detail tree loads, default expansion is root only.
- Create / rename / delete (note + empty folder) round-trip.
- Non-empty folder delete is refused with the expected message.
- External `mkdir` in the root surfaces in the tree within 1 second.
- Image tools panel: scan + migrate + Typora config check.
- Typora not installed: open falls back to `NSWorkspace.open` with no prompt.

## Wordbook (ADR-013)

- Cmd+Space → type `word` → first row "Start Review · N due" → Return → same-panel review.
- Cmd+Space → type `word review` → first row "Start Review · N due" → Return → same-panel review.
- During review: 1 (Known) / 2 (Fuzzy) / 3 (Unknown) grades; Space reveals/advances (search field empty).
- After completion: "Done for today" message; Esc returns home.
- Click settings gear during review → Settings opens → close Settings → review state preserved.
- Cmd+Space hide → Cmd+Space show → **same word continues**; progress numbers unchanged.
- Settings → Modules disable Wordbook → `word` trigger stops returning Wordbook rows → re-enable → rows return.

## v0.2 App Search (ADR-015)

- [ ] Search `微信` → WeChat row 1.
- [ ] Search `wx` / `weixin` / `wechat` → WeChat row 1.
- [ ] Search `vsc` → Visual Studio Code row 1.
- [ ] Search `ps` → Photoshop in top 3 (if installed).
- [ ] Single character `a` still lists prefix matches.

## v0.2 Wordbook Daily Plan (ADR-016)

- [ ] Detail home shows progress card; counts match DB.
- [ ] Mock +1 day → `daily_new_seen` resets.
- [ ] After 5 Unknown grades → fewer new cards in next 5 draws.
- [ ] CSV import 100 rows with duplicates → toast shows imported/skipped counts.
- [ ] Manage view Esc → home state; Esc again → launcher home list.

## v0.2 Detail UX

- [ ] Translate: zh-Hans / en / ja / ko chips switch target and re-translate.
- [ ] Clipboard: Pinned segment filters pinned only.
- [ ] Todo: Today / Upcoming / Completed tabs switch lists.

## In-Panel Settings (ADR-014)

- Gear icon visible at search bar trailing edge; hover darkens icon.
- Click opens Settings (same instance as menu bar / Cmd+,).
- Gear does not receive Tab focus.

## Round 3 (v0.3)

- [ ] Notes: `[Tree | Map]` segment switches in-panel; no sheet.
- [ ] Mind Map: double-click folder expands/collapses; double-click note opens Typora.
- [ ] Mind Map: Esc returns to launcher home (not trapped).
- [ ] Wordbook session: three buttons 不认识 / 认识 / 已学过; shortcuts 1/2/3.
- [ ] 已学过 skips answer reveal and jumps to next card.
- [ ] Done state shows `Continue · N more` with real N.
- [ ] Search bar: single character shows "继续输入以搜索…".
- [ ] Wordbook Manage: scroll to bottom loads next page; right-click Edit/Delete/Reset work.
- [ ] Settings Modules: rapid toggles debounce to one write (~200ms).
- [ ] Clipboard Image entries copy image bytes to pasteboard.
- [ ] Snippets: double-click copies snippet; ⌘E edits.
- [ ] Todo tabs show counts when non-zero.
- [ ] App search "微信" surfaces WeChat first row.

## Workbench Project Workspace (manual)

- [ ] Capture from Home CREATE with active IDE project → `workbench-activity.json` v2 entry includes `projectIdentity.stableProjectID` and `sourceKind: home`.
- [ ] `cap clip todo` shows preview row only while typing; Return executes once and writes activity + link (when project context exists).
- [ ] `proj work` / `proj open` preview → Return opens Current Project detail (not global search).
- [ ] `proj links` / `proj resume` / `proj capture` preview disabled-filtered rows; Return executes resume/capture/open workspace fallback.
- [ ] `attach clip` / `attach sel` with Snippets disabled → disabled preview row or status; no snippet draft written.
- [ ] Home shows **Continue project workspace**, latest draft, **Review linked** row when links exist.
- [ ] Project activity **not** in global top 8 still appears on Home / `proj recent` / detail (stableProjectID query).
- [ ] Recent draft row (Return) resumes snippet/quicklink/todo without re-capturing.
- [ ] Detail section order: header → Quick capture → **Linked items** → Recent activity (buttons) → Project actions.
- [ ] Detail activate shows loading placeholder immediately; no stale previous-project content.
- [ ] `proj recent` todo capture row (Home / command / detail) Return fills search with `t …` query — not silent noop.
- [ ] `proj recent` snippet/quicklink row Return opens module detail — same planner result on Home, command preview, and detail.
- [ ] Recorded (non-openable) activity rows show consistent subtitle; Return shows status hint (not silent noop).
- [ ] `proj recent` / `proj links` with no data show empty-state subtitle (not generic workspace fallback only).
- [ ] `proj status` shows stableProjectID + activity/link counts; Return displays full status message.
- [ ] Legacy activity-only data: empty or partial `workbench-links.json` → **Review linked** / `proj links` appear after first Home or detail load (lazy backfill).
- [ ] **Old data upgrade matrix:**
  - [ ] v1 / unversioned `workbench-activity.json` → v2 migrate; resumes work
  - [ ] v2 activity + empty `workbench-links.json` → links backfill on first read
  - [ ] v2 activity + partial links (other projects only) → current project links backfill
  - [ ] `draftPrepared` project todo/note/quicklink activity → appears in link index after backfill
  - [ ] Same entity re-captured with updated title → single link row (dedupe), latest title/subtitle
  - [ ] Duplicate links from title/subtitle drift → dedupe leaves one row per entityID
- [ ] Note activity row opens Notes module — does not promise opening a specific note path.
- [ ] Disable Snippets/Quicklinks/Todo/Notes → rows disappear consistently from Home, `proj links/recent/capture`, and detail.
- [ ] Legacy v1 / unversioned `workbench-activity.json` migrates to v2 and resumes still work.

## Recorded Product Review Additions

- First-run flow is understandable without reading docs.
- Spotlight conflict guidance is clear if Command+Space registration fails.
- Empty states explain what the user can do next.
- Error states suggest recovery rather than exposing internal wording.
- Search result ordering feels intuitive for the first 3 rows.
- Detail views feel visually related to the launcher rather than bolted on.
- Back/Esc behavior is consistent across all detail surfaces.
- Dense lists remain readable on both a 13-inch and larger display setup.
- Mixed-language input feels natural with Chinese and English queries.
- The app still feels coherent after enabling default-off modules.

## Issue Logging Rules

- Record each issue with repro steps, expected behavior, actual behavior, severity, and affected module.
- Distinguish product defects from optimization suggestions.
- Capture screenshot or recording timestamp for every medium-or-higher severity issue.
- Tag findings by lens when possible: functional, usability, keyboard, visual, performance, permissions.
