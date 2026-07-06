# Luma Module Handbook

This file is the source of truth for shipped user-visible module behavior. It replaces the older `Features/*/README.md` files.

## Global Rules

- Bare command: the trigger alone, such as `clip`, `n`, or `word`.
- Prefix search: trigger plus payload, such as `clip invoice`.
- Global search: unprefixed search with at least 2 characters.
- Return runs the primary action.
- Command+Return runs the first secondary action.
- Tab or Command+K opens the action panel.
- `help <trigger>` or `<trigger> ?` shows module help.
- Detail views open in the right column while Open Apps remains on the left.

## Active Modules

| Module | Triggers | Global Search | Default | Primary Surface |
| --- | --- | --- | --- | --- |
| Apps / Open Apps | `app`, `apps`, `open`, `top` | Yes | On | Home, app launch, app focus |
| Clipboard | `clip`, `clipboard` | Yes, capped | On | Clipboard detail/history |
| Commands | `cmd`, `reload`, `quit`, `settings`, scripted commands | Built-ins only | On | Built-in and local scripts |
| Notes | `n`, `note`, `notes` | No | On | Markdown workspace detail |
| Todo | `todo`, `td` | No | On | EventKit reminders |
| Translate | `tr`, `translate` | No | On | Translation detail/result |
| Wordbook | `word`, `wb` | No | On | Review/manage detail |
| Snippets | `s`, `snip`, `snippet` | Exact trigger expansion only | On | Snippet copy/paste/detail |
| Secrets | `secret`, `sec` | No values | On | Locked Keychain-backed vault |
| Records / Media | `m`, `rec`, `media` | No | On | Media log/search/detail |
| Window Layouts | `win`, `wl` | No | On | Focused-window layouts |
| Projects | `proj`, `p`, `project` | No | On | Project workspace |
| Quicklinks | `ql`, `quicklink`, configured exact triggers | Exact trigger only | On | URL template launcher/manager |
| Menu Bar Search | `mb`, `menu` | No | On | Active-app menu item search |
| Kill Process | `kill` | No | On | Quit/relaunch GUI apps |
| Browser Tabs | `tab`, `tabs` | No | Off | Browser tab search |

Deferred source-retained module: Windows.

## Apps / Open Apps

- Empty home shows running regular apps in activation-recency order.
- Luma itself is excluded.
- Multi-window apps can expose child window focus rows.
- `app <query>` launches or focuses matching applications.
- `app top` surfaces memory-aware running app rows.
- Secondary actions include quit and copy app path where applicable.
- Open Apps window controls may require Accessibility guidance when AX is denied.

## Clipboard

- Bare `clip` opens Clipboard detail.
- Global search returns matching in-memory entries, capped to avoid flooding results.
- Each clipboard entry stores a precomputed `cachedSearchHaystack` for O(1) query matching; haystack is rebuilt on insert/text/kind changes.
- Return copies the selected entry back to the pasteboard.
- Detail supports pin, delete, clear recent/today, source filtering, and preview.
- Privacy filters skip secret-looking values, blocked bundle IDs, oversized entries, concealed/transient types, and password-manager sources.
- History caps are controlled by settings.

## Commands And Scripts

- Built-ins include Settings, reload modules, diagnostics, and quit.
- User scripts load from `commands.json` in Application Support.
- Scripts execute asynchronously and do not block panel dismissal.
- Template variables may use clipboard, selection, project, file, UUID, date/time, and related context.
- No stdout streaming UI in v1.

## Notes

- Bare `n` opens Notes detail.
- Prefix search uses the warm in-memory note index.
- Detail supports Tree/Map, root settings, create note/folder, Today/Recent/Pinned/Outline/Browse/Inbox chips, backlinks/diagnostics, and open in Typora or system editor.
- Markdown files/folders are canonical; `notes.json` is local config/cache.
- `n doctor` may scan deeper health data and is not on the keystroke hot path.
- Non-goals: in-app Markdown editor, renderer, multi-vault sync, required graph, AI writing, or full body search as the primary surface.

## Todo

- Todo is EventKit pass-through, not a Luma-owned task database.
- Bare `todo` opens Todo detail.
- Natural language capture can create reminders.
- Detail exposes due/today/upcoming/completed style views and completion actions.
- Permission denied states must be actionable.
- Non-goals: projects/GTD suite, custom recurrence engine, team/shared tasks, cloud service, or Notion replacement.

## Translate

- `tr <text>` translates text.
- Bare `tr` opens Translate detail.
- Uses system/Shortcut-backed translation where available and surfaces mapped errors.
- No translation memory, glossary management, or cloud account in v1.

## Wordbook

- Bare `word` opens Wordbook detail.
- `word review` starts review.
- Review remains in-panel; no desktop pet window.
- Uses existing Wordbot data where configured.
- Supports daily plan, mixed session, grading, manage flow, import/export where implemented.
- Non-goals: public dictionary product, chatbot tutor, CSV/chat-paste-first workflow, or independent floating review app.

## Snippets

- Bare `s` opens Snippets detail.
- `s <query>` searches snippets.
- Return copies expanded content.
- Command+Return or secondary action can paste/insert where AX allows.
- Exact trigger expansion in global search can expand and paste.
- Variables include date/time, UUID, clipboard, selection, project, file, filename, and caret/cursor alias.
- If AX is denied, paste actions degrade or show permission guidance; copy still works.

## Secrets

- Values are stored in Keychain and never shown in global search.
- Bare `secret` opens vault detail.
- Locked state shows unlock flow; unlocked state exposes labels and actions.
- Copy clears after the configured timeout if not overwritten.
- Non-goals: full password manager, browser autofill, TOTP, shared vaults, or website-login storage.

## Records / Media

- `m` or `rec` opens/searches media records.
- Capture syntax supports title/category/rating/status/tags where parsed.
- Detail supports category tabs, status filter, sorting, add/edit/delete, and CSV export.
- Return on search result opens entry detail.
- Tab opens the action panel; documented secondary actions may copy summaries.
- Non-goals: TMDB/OMDb/Google Books enrichment, posters, streaming integration, social/discovery, or episode-level tracking.

## Window Layouts

- `win` / `wl` lists layout presets for the focused window.
- Requires Accessibility for window movement.
- AX trust is cached for 5 s per module instance; `requestAccess` invalidates the cache.
- Includes common halves/thirds/center/fullscreen style layouts.
- Non-goals: persistent multi-window layout manager or daemon.

## Projects / Current Project / Workbench

- `proj` opens project workspace/detail.
- Project identity comes from configured roots, recent activity, links, and IDE/frontmost context.
- Workbench actions convert selection/clipboard/context into note, todo, snippet, quicklink, or project links.
- Project context appears through commands/detail/template expansion, not empty home rows.
- No background disk scan on each query.

## Quicklinks

- `ql` opens manager.
- Configured exact first-token triggers launch URL templates.
- Variables expand through the snippet variable engine; values are percent-encoded.
- Quicklinks do not fuzzy-match globally.
- Persistent deletes require confirmation.

## Menu Bar Search

- `mb <query>` searches cached active-app menu items.
- Accessibility is required.
- AX traversal happens in bounded cache refreshes, not per keystroke.
- Diagnostics explain stale/denied/unavailable states.

## Kill Process

- `kill <query>` searches running regular GUI apps, excluding Luma.
- Cold cache refresh shows an informational "Refreshing process list..." row rather than silent empty results.
- Return sends normal Quit.
- Secondary actions include Force Kill and Relaunch where available.
- Force-kill and guarded system app operations require confirmation or elevated interaction.
- The module excludes daemons and raw signal management.

## Browser Tabs

- Default-off because AppleScript/Automation prompts are sensitive.
- `tab <query>` searches cached tabs for supported browsers.
- First cold query may return no tab rows while background refresh runs.
- Automation denied/timeout/degraded states surface as diagnostics.
- Browser adapters must not run AppleScript on the keystroke hot path.
