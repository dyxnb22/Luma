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

## Module failure taxonomy (P2.2)

Shared diagnostic kinds (`ModuleDiagnostic.kind`): `permissionRequired`, `degraded` (warming/partial), `error`, `timeout`. Dispatcher may also synthesize `module.warming` before targeted warmup completes (`QueryDispatcher`).

| Taxonomy | User-visible shape | When |
| --- | --- | --- |
| **Permission required** | Informational diagnostic row or thrown `ModuleError.permissionRequired` on perform | AX/EventKit/Automation denied on gated action |
| **Warming / degraded** | `ModuleDiagnostic.degraded` or dispatcher `module.warming` row | Cold cache, memory-top SWR, menu tree not ready |
| **Onboarding** | Actionable result row (open detail / Settings) | Data source not configured (Notes root unset) |
| **Timeout** | `ModuleDiagnostic.timeout` informational row | Module `handle` exceeds `queryTimeout` |
| **Empty acceptable** | Module-specific open-detail or no-match row; not a failure | Clipboard empty search, Apps no match |

### MVP P0 modules (C-FAIL-005)

| Module | Permission | Warming | Onboarding | Timeout | Empty OK |
| --- | --- | --- | --- | --- | --- |
| **Apps** | N/A in handle | `app top` → degraded "Memory usage cache warming" when cache cold | N/A | Via dispatcher | Global search no-match |
| **Clipboard** | `pasteEntry` → `.permissionRequired` when AX denied | N/A (store loaded at warmup) | N/A | Via dispatcher | `clip` empty → open-detail row |
| **Notes** | N/A in handle | Index rebuild on FSEvents | No root → "Choose a Notes root folder" | Via dispatcher | `n` search no matches |

Parked modules: record behavior in `MODULE_MATRIX.md`; do not change in P2.2 unless listed above.

## Lifecycle contract exceptions (P2.3)

Documented deviations — **do not fix in P2** without explicit scope expansion:

| Module | Exception | Status |
| --- | --- | --- |
| **Windows** | `handle()` calls `CGWindowListCopyWindowInfo`; deferred, not registered | Parked |
| **Kill Process** | No explicit `teardown`; refresh tasks | Parked, default-off |
| **Wordbook** | `perform` throws `unsupportedAction`; review in detail | Parked, default-off |

P0 modules (Apps, Clipboard, Notes): `handle()` memory-only proxy tests in `ModuleHandleContractTests` + `scripts/scan_handle_memory_only.sh`.

## MVP default-on modules (fresh install)

Apps, Clipboard, Snippets, Quicklinks, Todo, Translate, Notes.

## Default-off modules (enable in Settings)

Commands, Media, Browser Tabs, Menu Bar Search, Window Layouts, Wordbook, Secrets, Kill Process, Projects.

Home guide (empty query, right column) lists discoverable commands for **enabled** modules only; Apps appears in the guide. Disabled modules do not show guide rows.

## Registration status (P2.1)

| Status | Modules | Meaning |
| --- | --- | --- |
| **Registered** | All Active Modules rows below except Windows | In `ModuleRegistry.allBundles`; warmup/enablement applies |
| **Deferred** | Windows | Source in `BuiltInModules.makeDeferred()`; **not** in `ModuleRegistry.allBundles` |
| **Parked** | Media, Secrets, Window Layouts, Menu Bar Search, Kill Process, Browser Tabs, Wordbook, Projects; complex Workbench/Capture | Registered but default-off; not MVP main path per `MVP_SCOPE.md` |

## Active Modules

| Module | Triggers | Global Search | Default | Reg. | Primary Surface |
| --- | --- | --- | --- | --- | --- |
| Apps / Open Apps | `app`, `apps`, `open`, `top` | Yes | On | registered | Home, app launch, app focus |
| Clipboard | `clip`, `clipboard` | Yes, capped | On | registered | Clipboard detail/history |
| Commands | `cmd`, `reload`, `exit`, `settings`, scripted commands | Built-ins only | Off | registered | Built-in and local scripts |
| Notes | `n`, `note`, `notes` | No | On | registered | Markdown workspace detail |
| Todo | `todo`, `td` | No | On | registered | EventKit reminders |
| Translate | `tr`, `translate` | No | On | registered | Translation detail/result |
| Wordbook | `word`, `wb` | No | Off | registered | Review/manage detail |
| Snippets | `s`, `snip`, `snippet` | Exact trigger expansion only | On | registered | Snippet copy/paste/detail |
| Secrets | `secret`, `sec` | No values | Off | registered | Locked Keychain-backed vault |
| Records / Media | `m`, `rec`, `media` | No | Off | registered | Media log/search/detail |
| Window Layouts | `win`, `wl` | No | Off | registered | Focused-window layouts |
| Projects | `proj`, `p`, `project` | No | Off | registered | Project workspace |
| Quicklinks | `ql`, `quicklink`, configured exact triggers | Exact trigger only | On | registered | URL template launcher/manager |
| Menu Bar Search | `mb`, `menu` | No | Off | registered | Active-app menu item search |
| Kill Process | `kill`, `quit`, `k` | No | Off | registered | Quit/relaunch GUI apps |
| Browser Tabs | `tab`, `tabs` | No | Off | registered | Browser tab search |
| Windows | (none registered) | No | Deferred | **deferred** | Not registered — see deferred note below |

**Quit vs exit:** bare `quit` / `kill` / `k` targets Kill Process (quit frontmost GUI app). Bare `exit` exits Luma when the Commands module is enabled. `cmd quit` also exits Luma from command mode.

**MVP default install:** Kill Process and Commands are **off by default**, so bare `quit`, `kill`, `k`, and `exit` do **not** respond until you enable those modules in Settings. To quit Luma without enabling Commands, use the menu bar **⌘Q** or Luma → Quit.

Deferred source-retained module: **Windows** (`BuiltInModules.makeDeferred()`). Not registered in `ModuleRegistry.allBundles`; manifest `defaultEnabled: false`; `handle()` must not ship on the hot path until warm-cache + tests land.

## Snippets vs Quicklinks vs Commands

| Surface | Purpose | Trigger model | Storage |
| --- | --- | --- | --- |
| **Snippets** | Expand text templates into paste/insert | Exact trigger match on Return (e.g. `sig`) | Local snippet store |
| **Quicklinks** | Open URLs from templates | Exact first-token triggers (e.g. `gh`) | `quicklinks.json` |
| **Commands** | Run local scripts + built-ins (`settings`, `exit`, `doctor`) | `cmd` prefix or bare built-in | `commands.json` |

Snippets never open URLs; Quicklinks never run shell scripts; Commands never store snippet bodies. Enable Commands in Settings when needed.

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

- Built-ins include Settings, reload modules, diagnostics, and exit (`exit` bare or `cmd quit`).
- **Recovery (default install):** menu bar **Run Doctor…** and **Export Diagnostics…** call `AppHostService` directly — not gated by Commands.
- `cmd doctor` / `commands doctor` runs the global doctor scan when Commands is enabled (AX, EventKit, config checks, hotkey registration, corrupt configs, latency p95). Bare global `doctor` without the commands prefix does **not** run doctor checks.
- Notes without a configured root shows onboarding row ("Choose a Notes root folder"); configure via Settings → Notes or Notes detail.
- Projects with no scan roots or matches shows onboarding row; configure via Settings → Projects.
- `cmd export-diagnostics` writes the same redacted `~/Library/Logs/Luma/diagnostics.json` as menu bar export when Commands is enabled (`RecoveryDiagnosticsCollector` + `DiagnosticsExport`).
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
