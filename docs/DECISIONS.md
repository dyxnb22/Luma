# Luma Decision Log

This compact log replaces the individual ADR files. Older superseded route and dashboard notes were folded into the summaries below.

## Active Product Decisions

| ID | Decision | Status |
| --- | --- | --- |
| D-001 | Use Swift 6 + AppKit for launcher UI; SwiftUI is allowed for Settings/About. | Active |
| D-002 | Pre-instantiate the launcher panel at app launch. | Active |
| D-003 | Built-in modules are in-process actors behind `LumaModule`. | Active |
| D-004 | v1 modules ship in the app; no public plugin runtime. | Active |
| D-005 | Use Carbon `RegisterEventHotKey` for the global launcher chord. | Active |
| D-006 | Route C command-first unified launcher is the active UI model. | Active |
| D-007 | Empty-query home is Open Apps left + guide/detail right. | Active |
| D-008 | Module details stay in-panel and must preserve keyboard exit/editability. | Active |
| D-009 | Home dashboard/cards/setup/recent/project rows are not active. | Active |
| D-010 | Accessibility guidance is lazy and path-specific. | Active |
| D-011 | Hot paths are stale-while-revalidate and memory-only. | Active |
| D-012 | Stabilization MVP: seven default-on modules (Apps, Clipboard, Snippets, Quicklinks, Todo, Translate, Notes); six expert modules default off. | Active |
| D-013 | IME composition gate: no query dispatch while marked text is active. | Active |
| D-014 | Bare `quit` resolves to Kill Process; Luma exit is `exit` (Commands module). When Kill Process / Commands are off (MVP default), bare `quit` and `exit` do not respond — use menu bar ⌘Q. | Active |
| D-015 | External actions use perform-then-dismiss; failures keep the panel open with status feedback. | Active |
| D-016 | Notes `.open` paths must resolve under configured root; local files use `openLocalFileURL` after containment check. | Active |
| D-017 | `WorkspaceService.openURL` allows `http`/`https`/`mailto` only; `file://` requires `openLocalFileURL`. | Active |
| D-018 | Singleton JSON configs use `JSONConfigPersistence` quarantine on decode failure; doctor lists corrupt files. | Active |
| D-019 | Global hotkey re-registers on Space change; Open Apps refreshes after system wake. | Active |
| D-020 | Window Layouts (`win`/`wl`) remains default-off; no hot-path registration until warm-cache ships. | Active |
| D-021 | Launcher detail presentation and keyboard routing live in `LauncherDetailPresenter` / `LauncherKeyboardDispatcher`. | Active |
| D-022 | Existing installs migrate `enabledModules` to schema v2 (MVP defaults + pinned expert modules). | Active |

## Architecture Decisions

- AppKit owns the primary launcher because focus, keyboard routing, and panel positioning must be predictable.
- The launcher panel is ordered out when hidden rather than recreated.
- `ModuleHost` owns enabled modules, warmup state, teardown, and query context.
- `QueryDispatcher` handles global and targeted query routing with timeouts and diagnostics.
- `QuerySnapshotCache` (query-side, top-N snapshots, stale-while-revalidate) is separate from `UsageResultCache` (post-action single-row storage); only **secrets** and **snippets** are excluded from query cache (clipboard may be cached in mixed global snapshots).
- Detail views live in `LumaApp`; module actors own data and actions.
- Scripted commands are local JSON-backed commands, not a plugin marketplace.

## Route History

- Route A, "pure launcher convergence", is historical.
- Route B, dashboard/widget single-window, is historical.
- Route C supersedes both: command-first list, no dashboard home, split home guide/detail, same-panel module detail.
- Historical ADR text that mentioned cards, dashboard rows, home project context, or setup rows is no longer authoritative.

## Module Decisions

- App Search supports fuzzy matching, pinyin, aliases, running-app indicators, and Open Apps focus.
- Clipboard v1 is local history with strict privacy filters and bounded retention.
- Todo v1 is EventKit pass-through.
- Wordbook v1 is in-panel review/manage, not a separate pet window.
- Snippets and Secrets are separate modules because their security models differ.
- Notes is a Markdown workspace manager; ADR-019-style scope is active.
- Media/Records is a local personal log, not a metadata/social product.
- Quicklinks use exact first-token triggers and URL templates.
- Menu Bar Search uses a cached AX tree.
- Kill Process targets regular GUI apps only.
- Browser Tabs is default-off and stale-while-revalidate.
- Current Project / Workbench context appears through `proj`, detail, links, and template expansion, not home rows.

## Superseded Or Historical Decisions

The following old decisions are retained here only as context:

- Launcher convergence and dashboard widget routes were superseded by Route C.
- Dashboard cards and feature tiles were removed from the active home model.
- Notes graph/product ambitions were reduced to Markdown workspace management.
- Wordbook dedicated review window was superseded by in-panel review.
- Todo/Wordbook/Media early dashboard registration notes were superseded by command-first module rows.
- Browser Tabs originally allowed a blocking empty-cache refresh; it now returns cached/empty immediately and refreshes in the background.

## Decision Change Rule

If a future change alters launcher home, panel geometry, keyboard contract, module visibility, persistence semantics, privacy defaults, or hot-path performance, update this file and the relevant section in `docs/ENGINEERING.md` or `docs/MODULES.md` in the same change.
