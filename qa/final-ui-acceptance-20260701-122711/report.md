# Luma Final UI Acceptance Report

## Remediation (2026-07-02)

The following P1/P2 findings from the 2026-07-01 pass were addressed in code and docs:

| Area | Fix |
|---|---|
| Tab / action panel | Tab no longer selects search text; opens secondary actions. Shift+Tab closes the panel. |
| Empty query stale results | Esc and empty sync clear results and restore Home immediately. |
| Bare commands | `todo`, `word review`, `s new`, `app top`, module bare-open-detail rows work. |
| Detail layout cropping | Horizontal scroll toolbars/footers; table truncation and tooltips across Clipboard, Notes, Quicklinks, Translate, Records, Auto Workflow. |
| Detail + search confusion | Search field read-only in detail with module placeholder; Esc restores suspended query. |
| Single-char global search | No module fan-out below 2 characters; hint explains minimum. |
| IME help | `help <trigger>` promoted in hints, empty states, and module help lines. |
| Permissions / setup | Home setup rows for Accessibility and modules; permission banner Settings button. |
| Autoworkflow tasks | Optimistic task row on start; stale error cleared. |
| Kill Process | Confirmation before quit/force kill. |
| Settings | Apply feedback, path wrapping (General/Wordbook), default-off badges, Activity labels. |
| Edit shortcuts | Command+A/C/V/X/Z and undo/redo globally via `LumaStandardEditShortcuts`. |

Re-run `./scripts/build_app.sh` and `./scripts/verify_manual_qa.sh` after pulling these changes. Screenshots in this folder remain the **pre-remediation** baseline.

## Verdict
- Conditional Pass
- Luma is basically usable for the owner's daily launcher workflows, but it is not ready for broader trial or release because multiple visible modules fail documented commands and several detail views show severe layout/cropping problems.

## Environment
- macOS version: macOS 26.3.1 (25D2128)
- Display setup: Built-in Liquid Retina XDR Display, 3024 x 1964 Retina, single display
- Input method: com.apple.inputmethod.SCIM.ITABC
- Luma build path: /Users/diaoyuxuan/Luma/build/Luma.app
- Date/time: 2026-07-01 12:27-12:55 Asia/Hong_Kong
- Scripts run:
  - ./scripts/build_app.sh: pass, app signed and restarted
  - ./scripts/verify_manual_qa.sh: pass, 536 Swift tests passed; warnings in AutoworkflowDetailView for unused syncTaskListStatus calls
  - ./scripts/qa/autoworkflow_preflight.sh: pass, cc-loop available on shell PATH and Luma GUI PATH, QA source /tmp/luma-aw-qa-source exists, Auto Workflow enabled
  - ./scripts/qa/autoworkflow_collect.sh: pass, collected /Users/diaoyuxuan/Luma/qa/autoworkflow-ui-20260701-125455

## Coverage
| Area / Module | Commands tested | Tested? | Result | Notes | Screenshot path |
|---|---|---:|---|---|---|
| Launcher Home | Command+Space, Esc | yes | conditional | Hotkey works through System Events; Esc closes/clears. Home rows visible but right-side tags and bottom hints are very low contrast. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/launcher-home.png |
| Search Results | safari, long URL no-result | yes | conditional | Fast results; no-result state works but sparse. Long text fits search field but result/detail panes later crop. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/search-results-safari.png |
| Action Panel | Tab on result | partial | fail | Tab selected search text instead of opening secondary actions. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/action-panel-tab-attempt.png |
| Settings General | Settings UI | yes | pass | Opens and readable; long data path wraps awkwardly. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-general.png |
| Settings Modules | Modules | yes | conditional | Current state shows many modules enabled; default-off modules are not marked per module. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-modules-top.png |
| Settings Clipboard | Clipboard | yes | pass | Controls readable; Apply lacks unsaved/saved feedback. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-clipboard.png |
| Settings Translation | Translation | yes | pass | Simple and readable. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-translation.png |
| Settings Wordbook | Wordbook | yes | conditional | Database path wraps hard; Reset not executed. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-wordbook.png |
| Settings Secrets | Secrets | yes | pass | Controls readable; Apply feedback missing. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-secrets.png |
| Settings Accessibility | Accessibility | yes | conditional | Granted state clear, but disabled button still says "Grant Access...". | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-accessibility-granted.png |
| Settings Activity | Activity | yes | conditional | Graphs visible; module counts use raw ids and cramped inline text. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-activity.png |
| Settings Auto Workflow | Auto Workflow | yes | pass | Availability accurately shows source and cc-loop available. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-autoworkflow.png |
| Settings Developer | Developer | yes | pass | Latency HUD toggle visible. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-developer.png |
| Apps | app, app top, safari | yes | fail | app search works; app top returns no results. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-app-top-no-results.png |
| Clipboard | clip, clipboard detail | yes | conditional | Detail opens and actions visible; long rows and detail content crop horizontally. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-clipboard-detail.png |
| Commands | help | partial | pass | help lists command-entry rows. '?' could not be reliably typed under current input method/tool path. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-commands-help.png |
| Notes | note | yes | conditional | Results and detail open; detail layout has severe white-panel style break and clipping. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-notes-detail-layout-break.png |
| Todo | todo | partial | fail | Bare command shows informational row only; no detail page reached. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-todo-bare.png |
| Translate | tr hello | yes | conditional | Detail opens; dependency error is friendly; layout crops to the right. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-translate-detail-failure.png |
| Wordbook | word review | yes | fail | Documented review command returns no matching results. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-word-review-no-results.png |
| Snippets | s, s new | yes | fail | Search works; s new returns no results; duplicate snippet rows are hard to distinguish. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-snippet-new-no-results.png |
| Secrets | secret | partial | pass | Locked state opens; did not unlock or copy sensitive data. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-secrets-locked.png |
| Records / Media | m, m log | yes | conditional | m alone no result; m log opens detail. New/Export exist in AX tree but are visually hard to find due cropping. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-records-detail.png |
| Window Layouts | win | partial | pass | Layout options list; no layout executed to avoid moving user windows. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-window-layouts-results.png |
| Projects | proj | partial | conditional | Finds Luma project; no manager/detail visible, Return would open project. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-projects-results.png |
| Quicklinks | ql | yes | conditional | Manager opens; long URL/template columns truncate heavily. Delete not executed. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-quicklinks-detail.png |
| Menu Bar Search | mb fold | yes | fail | Shows stale Home/Open Apps content rather than menu results or diagnostic. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-menu-bar-search-home-stale.png |
| Kill Process | kill | partial | pass | Lists GUI apps, excludes Luma, shows memory. Quit/Force Kill not executed. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-kill-process-results.png |
| Browser Tabs | tab github | partial | conditional | No results and no diagnostic explaining no tabs, no permission, or empty cache. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-browser-tabs-no-results.png |
| Auto Workflow | aw, start, stop, hide/reopen | yes | conditional | Real start/stop works and no cc-loop remains; task list does not show newly running task, and starting initially preserves stale error. | /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/autoworkflow-running.png |

## Findings

### P0 Blocking
None found. The app launches, hotkey can open/close the panel, Settings opens, and several modules are usable.

### P1 Major
#### 1. Detail views and long result rows are horizontally cropped across modules
- Repro steps: Open launcher; run clip -> Return, note -> Return, tr hello -> Return, m log -> Return, ql -> Return.
- Actual result: Multiple detail views extend/crop at the right edge; Notes and Translate show large white AppKit/SwiftUI-looking regions that visually break from the launcher shell; long Clipboard rows run off visually.
- Expected result: Detail views should fit within the launcher panel with stable margins, no clipping, and consistent background/style.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-notes-detail-layout-break.png
- Impact scope: Clipboard, Notes, Translate, Records, Quicklinks, Auto Workflow; overall product polish.
- Suggested direction: Audit detail container sizing, intrinsic content width, scroll view constraints, and table column compression. Verify at the actual 3024 x 1964 Retina scaled workspace.

#### 2. Tab/secondary action keyboard path does not work reliably
- Repro steps: Search safari; press Tab.
- Actual result: Search text is selected; action panel does not open.
- Expected result: Tab should open secondary actions for selected result without stealing text selection.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/action-panel-tab-attempt.png
- Impact scope: Keyboard-first promise; all modules using secondary actions.
- Suggested direction: Revisit first-responder/key routing between NSSearchField and result list; add recorded UI test for Tab from search-focused state.

#### 3. Empty query can show stale previous search results
- Repro steps: Type a single-character query such as a; press Esc to clear.
- Actual result: Search field becomes empty but APPS results remain for a moment/state instead of Home.
- Expected result: Empty query immediately restores Home sections.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-empty-query-stale-results.png
- Impact scope: Launcher state model, user trust in search mode vs home mode.
- Suggested direction: Tie rendered content strictly to effective query state; clear dispatcher snapshot when query becomes empty.

#### 4. Documented Apps memory command app top returns no results
- Repro steps: Type app top.
- Actual result: No matching results.
- Expected result: Running apps sorted by resident memory.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-app-top-no-results.png
- Impact scope: Apps module, documented QA checklist.
- Suggested direction: Verify parser path for app top vs app <query> and current module enablement.

#### 5. Wordbook review command does not surface review entry
- Repro steps: Type word review.
- Actual result: No matching results.
- Expected result: Start Review row or a clear "done/no words" state.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-word-review-no-results.png
- Impact scope: Wordbook core workflow.
- Suggested direction: Restore targeted route for word review and add empty/done fallback row.

#### 6. Snippets creation command s new returns no results
- Repro steps: Type s new.
- Actual result: No matching results.
- Expected result: Create snippet row or Snippets detail/editor entry.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-snippet-new-no-results.png
- Impact scope: Snippets creation and trigger-expansion onboarding.
- Suggested direction: Make creation command explicit and visible even with empty store/query.

#### 7. Menu Bar Search returns stale Home content instead of result or diagnostic
- Repro steps: Type mb fold.
- Actual result: Shows Home/Open Apps content under a Menu Bar Search command header.
- Expected result: Matching menu items, or a clear Accessibility/frontmost-app/no-match diagnostic.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/finding-menu-bar-search-home-stale.png
- Impact scope: Menu Bar Search reliability and permission UX.
- Suggested direction: Prevent targeted command views from reusing Home snapshot; surface explicit empty/error rows.

#### 8. Todo detail workflow is not reachable from bare todo
- Repro steps: Type todo.
- Actual result: "Nothing due today" informational row; Return is "Information only".
- Expected result: Todo detail page or a clear route to Today/Upcoming/Completed tabs per QA checklist.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-todo-bare.png
- Impact scope: Todo module discoverability and management.
- Suggested direction: Add/restore an Open Todo detail row for bare todo/t.

#### 9. Auto Workflow task list does not show the newly running task
- Repro steps: aw -> Start Workflow with valid goal/repo.
- Actual result: Status card shows task-AE78C337-85E running with PID, but TASKS list still shows older tasks only.
- Expected result: Newly started task appears in task list immediately and remains visible after hide/reopen/stop.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/autoworkflow-running.png
- Impact scope: Auto Workflow status recovery and user confidence.
- Suggested direction: Insert selected/current task into visible list immediately after start and reconcile list/status snapshots.

### P2 Polish
#### 1. Default-off modules are not clearly marked in Settings Modules
- Repro steps: Settings -> Modules.
- Actual result: Generic explanatory text says default-off modules exist, but Commands/Records/Browser Tabs/Auto Workflow are not labeled per-row.
- Expected result: Each default-off module should have a visible "Default off" or similar badge.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-modules-top.png
- Impact scope: Settings clarity.
- Suggested direction: Add per-module metadata chips without changing enabled state.

#### 2. Activity page exposes raw module ids and cramped text
- Repro steps: Settings -> Activity.
- Actual result: "luma.apps: 93 · luma.windows..." is raw and dense.
- Expected result: Human module names in a readable list/table.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-activity.png
- Impact scope: Settings polish.
- Suggested direction: Use display names and line wrapping/grid layout.

#### 3. Accessibility granted state still shows disabled "Grant Access..."
- Repro steps: Settings -> Accessibility with permission granted.
- Actual result: Status is Granted, but button still says Grant Access....
- Expected result: Button hidden or renamed "Open System Settings..." if useful.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-accessibility-granted.png
- Impact scope: Permission UX.
- Suggested direction: Make button state match permission state.

#### 4. Settings keyboard/window behavior is inconsistent
- Repro steps: Open Settings; press Command+W; try PageDown in Auto Workflow settings.
- Actual result: Command+W did not close Settings; PageDown did not scroll the right pane.
- Expected result: Standard macOS window and scroll keyboard behavior.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/settings-autoworkflow.png
- Impact scope: Settings accessibility and keyboard use.
- Suggested direction: Check key-equivalent/window responder handling and scroll view focus.

#### 5. Browser Tabs no-result state lacks diagnostic context
- Repro steps: Type tab github.
- Actual result: No matching results.
- Expected result: Explain whether no browsers/tabs, no Automation permission, stale cache, or no match.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-browser-tabs-no-results.png
- Impact scope: Default-off/external dependency UX.
- Suggested direction: Add diagnostic rows for enabled-but-empty/permission states.

#### 6. Auto Workflow keeps stale error while starting
- Repro steps: Trigger empty-goal error, fill goal/repo, click Start.
- Actual result: Button says Starting... but status card still shows "Goal and Repo are required" until running state arrives.
- Expected result: Clear previous error immediately and show Starting/Initializing.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/autoworkflow-starting-after-5s.png
- Impact scope: Auto Workflow trust.
- Suggested direction: Reset error state synchronously before async start.

#### 7. No-result state is too sparse for command workflows
- Repro steps: Type a long unknown query.
- Actual result: Only "No matching results."
- Expected result: Add command suggestions or "press Esc to clear" context when appropriate.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/no-results-long-url.png
- Impact scope: Search clarity.
- Suggested direction: Add lightweight suggestions without cluttering the hot path.

#### 8. Kill Process footer advertises Force Kill without visible confirmation copy
- Repro steps: Type kill.
- Actual result: Footer shows Command+Return Force Kill.
- Expected result: Dangerous action should visibly imply confirmation/protection before execution.
- Screenshot path: /Users/diaoyuxuan/Luma/qa/final-ui-acceptance-20260701-122711/screenshots/module-kill-process-results.png
- Impact scope: Safety perception.
- Suggested direction: Add confirmation hint or require action panel confirmation in visible copy.

## UI Consistency Review
The launcher shell feels unified on Home and simple result lists, but detail views split into several visual languages. Clipboard, Notes, Translate, Records, Quicklinks, and Auto Workflow use different densities, table styles, section cards, and backgrounds. Notes and Translate especially look like embedded development panels rather than the same product surface. The common top search bar helps, but it is not enough to hide the style discontinuity.

The biggest visual issue is responsive sizing: the panel repeatedly appears too wide or crops right-side content. Secondary labels and footer hints are often too low contrast. Long text, long paths, URLs, and tables need a consistent truncation/wrapping strategy.

## Settings Review
All Settings pages opened. General, Clipboard, Translation, Secrets, Developer, and Auto Workflow are broadly understandable. Modules needs per-row default-off marking. Activity should not expose raw ids. Accessibility granted state should remove or rename the disabled Grant Access button. Settings keyboard behavior needs cleanup: Command+W did not close the window and PageDown did not scroll the Auto Workflow pane.

## Module Review
- Apps: normal search works; app top fails.
- Clipboard: strong feature surface, but long rows/detail layout crop. Pin/Delete/Create Snippet buttons visible; destructive delete not executed.
- Commands: help works; ? was not reliably testable under current input method/tool injection.
- Notes: search and tree detail open, but detail layout is not release-polished.
- Todo: bare state works as status, but management detail page was not reachable.
- Translate: dependency failure is friendly; layout crops.
- Wordbook: review command fails.
- Snippets: search works; creation command fails; duplicate rows hard to distinguish.
- Secrets: locked state is safe; unlock/copy not tested to avoid sensitive data exposure.
- Records: m log opens detail; m alone no result; New/Export discoverability poor.
- Window Layouts: list works; execution not performed to avoid moving windows.
- Projects: finds Luma; opening not performed to avoid side effects.
- Quicklinks: manager opens; URL columns truncate.
- Menu Bar Search: targeted query shows stale Home content.
- Kill Process: list works and excludes Luma; quit/force kill not executed.
- Browser Tabs: enabled but no diagnostic on empty/no-match state.
- Auto Workflow: real start/stop works, no residual cc-loop process; list/status sync and transient state need polish.

## Command Coverage
Tested successfully or partially: Command+Space, Esc, safari, app, clip, help, note, todo, tr hello, secret, m, m log, proj, ql, kill, tab github, win, aw.

Failed or did not meet expected behavior: app top, word review, s new, mb fold, Tab secondary actions, empty-query clear after prior single-character search.

Not fully executed due safety/side-effect constraints: Return to launch/activate apps, Quicklink URL opening, Kill Process quit/force kill/relaunch, Window Layout execution, Secrets unlock/copy, destructive Delete/Clear actions.

## Auto Workflow Result
Preflight passed: source path /tmp/luma-aw-qa-source exists, cc-loop is available on Luma GUI PATH, state root exists.

UI pass:
- Settings availability: pass.
- aw detail opens: pass.
- Empty goal/repo error: pass, user-facing.
- Real start: pass; task task-AE78C337-85E started with PID 67166.
- Running state: conditional; status card showed task ID, status, phase, iteration, PID, but task list did not show the new running task.
- Hide/reopen: pass for window restoration; detailed status restore looked stable enough after stop.
- Stop: pass; UI changed to Stopped and Resume appeared.
- Residual process check: pass; no cc-loop/claude process remained after stop.
- Resume: visible but not executed because the requested start/stop evidence was already collected and resume would re-launch provider work.
- Collect evidence: pass; /Users/diaoyuxuan/Luma/qa/autoworkflow-ui-20260701-125455/report.md.

## Product Readiness
- Daily self-use: yes, conditional. Core launcher, app search, clipboard, some managers, and Auto Workflow start/stop can be used by the owner with awareness of rough edges.
- Give to other people to try: not yet. The visible layout breakage and failed documented commands will look unreliable.
- Must fix before release: detail view cropping/responsive layout, Tab secondary actions, stale empty-query results, app top, word review, s new, mb fold, Todo detail route, Auto Workflow task-list sync.
- Can defer: Settings copy polish, raw Activity ids, Browser Tabs richer diagnostics, no-result suggestions, Quicklinks URL column polish, duplicate snippet row differentiation.
