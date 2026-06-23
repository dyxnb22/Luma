# Roadmap

Active route: Route B (Dashboard Widget Single Window) per ADR-007.

Implementation reference:

- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
- `docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md`
- `docs/strategy/DASHBOARD_WIDGET_POLISH_PLAN.md`

Route A (`docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`, `CONVERGENCE_EXECUTION_PLAN.md`) is preserved as historical reference only; do not implement against it without superseding ADR-007.

**Round 3 audit & taskbook** (2026-06-22): `docs/strategy/ROUND_3_AUDIT_AND_TASKBOOK.md` — v0.3 P0/P1 bugs, Mind Map inline, Wordbook three-button grade, PR split.

## Current Strategic Roadmap

| Version | Focus | Do | Do Not Do |
| --- | --- | --- | --- |
| 0.1 Daily self-use | Dashboard Widget shell | Route B single window, App Search sidebar, **Translate + Clipboard + Notes + Todo + Wordbook + Snippets + Secrets** core cards, **Media (trigger-only)**, Commands, same-panel details (Wordbook review in-panel per ADR-013), in-panel settings gear (ADR-014) | Calculator/Windows on dashboard, Plugin API, Notion-style TODO database, Floating Wordbook pet, ChatGPT-paste import, browser password autofill, Douban-style social/discovery features in Media, streaming integration, Media metadata fetch (TMDB/OMDb/Google Books) in v1 |

## Todo + Wordbook v0.1 (ADR-009)

- Todo: EventKit pass-through. Trigger `t `. No Luma-owned TODO database.
- Wordbook: full successor to TechWordPet.app. SQLite migrated to `~/Library/Application Support/Luma/Wordbook/wordpet.sqlite3` on first launch. Review opens in the launcher panel (ADR-013).

Reference: `docs/adr/009-todo-wordbook-v01.md`

## UI optimization (ADR-013 / ADR-014, 2026-06-22)

| Priority | Done | Item |
| --- | --- | --- |
| P0 | ✓ | Wordbook in-panel detail; settings gear; ADR-013/014 |
| P0 | ✓ | Remove `WordbookReviewPanel`; `openModuleDetail` bridge |
| P1 | ✓ | `PermissionBannerController`; panel-visible polling (3 s) |
| P1 | ✓ | `OpenAppsSidebarController` diff updates |
| P1 | ✓ | `LauncherEnvironment` + `ModuleLauncherHooks` (replaces `LauncherBridge`) |
| P1 | ✓ | Events/Commands default disabled |
| P1 | ✓ | `LauncherRootView` split (387 lines; coordinators + chrome) |
| P1 | ✓ | Card status event subscriptions (`*ChangeHub`) |
| P1 | ✓ | Notes 三件套：`NotesDetailView` + `NotesMindMapView` + `NotesImageToolsPanel` + `NotesDetailSheets` |
| P2 | ✓ | Shortcuts info popover, card badges, Settings SwiftUI rewrite, Wordbook「继续学」 |
| P2 | — | `BaseDetailContainer` unify (detail views already use it; Notes uses shared top bar) |
| P3 | ✓ | Typora `NSWorkspace.open`; gradient merge; `defaultCards` removed; Media opt-in (`defaultEnabled: false`) |

Reference: `docs/adr/013-wordbook-back-in-panel.md`, `docs/adr/014-in-panel-settings-entry.md`

## v0.2 P0 (2026-06-22)

| Item | Status |
| --- | --- |
| App search: localized names + aliases + pinyin + subsequence fuzzy (ADR-015) | ✓ |
| Wordbook daily plan + `WordbookSessionPlanner` + progress home (ADR-016) | ✓ |
| Wordbook manage + CSV import + TTS accent popover | ✓ |
| Bug: review session survives panel hide | ✓ |
| Bug: Settings Modules toggle debounce | ✓ |
| Bug: Tab skips gear/info buttons | ✓ |
| `ModuleLauncherHooks` replaces `LauncherBridge` | ✓ |
| Translate quick language chips | ✓ |

Reference: `docs/adr/015-app-search-fuzzy-pinyin.md`, `docs/adr/016-wordbook-daily-plan.md`

## Snippets + Secrets v0.1 (ADR-010)

- Snippets: local plaintext cheatsheet library. Trigger `s `. Dashboard green card.
- Secrets: Keychain developer-credential vault. Trigger `secret `. Dashboard gold card. Auto-clear pasteboard + idle re-lock.

Reference: `docs/adr/010-snippets-secrets.md`, `docs/strategy/SNIPPETS_SECRETS_PLAN.md`

## Media v0.1 (ADR-011)

- Lightweight local-only log for movies / TV / anime / games / books. Trigger `m ` (also `media `).
- Trigger-only: no dashboard card. Detail view via `m log` or manage row.

Reference: `docs/adr/011-media-tracker.md`, `docs/strategy/MEDIA_TRACKER_PLAN.md`
- Trigger-only — no dashboard card (8-slot ceiling preserved).
- One-line capture DSL (`m Oppenheimer movie 9`). Same-panel detail view for browse/edit. CSV export.
- No social features, no metadata fetch, no posters, no episode-level tracking, no streaming integration.

Reference: `docs/strategy/MEDIA_TRACKER_PLAN.md`

## Notes v0.1 (ADR-008)

Markdown file index and Typora launcher delivered in eleven implementation phases:

0. Demolish NotesGraph scaffolding
1. Real FSEventsService
2. NotesRootConfig + NotesTreeIndex
3. Rewrite NotesModule (memory-only handle)
4. Detail view
5. Create note + folder
6. Rename, delete, context menus
7. In-tree filter
8. Image tools panel
9. Wiki link jump + recent notes
10. Docs and verification

Reference: `docs/adr/008-notes-markdown-manager.md`, `docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md`
| 0.2 Stability | Persistence and settings | Namespaced Application Support persistence, interactive Settings, corrupt-data recovery, perf gates | XPC, runtime plugins, schema migration churn without need |
| 0.3 Command depth | Action depth | Secondary action chooser, better result grouping, stronger command ergonomics | Marketplace, JS/Lua runtime |
| 0.5 Architecture | Lifecycle and scale | ResidencyController, module lifecycle budgets, optional SQLite only if JSON files become insufficient | Premature XPC |
| 1.0 Maintainable app | Polish and distribution | Signed/notarized app, docs, repeatable release | Cloud sync, telemetry, broad public platform scope |

## Historical Phase Notes

These phase notes are preserved for context. Treat the current code and ADR-007 as authoritative when they differ.

## Phase 0: Skeleton

- Swift package and eventual Xcode project wiring.
- LSUIElement/accessory app setup.
- Carbon hotkey controller with Command+Space as the default.
- Menu bar item with Show Luma, Settings..., and Quit Luma.
- Pre-instantiated empty panel.
- Metrics hooks.
- `Features/` folder with per-module docs.

## Phase 1: Launcher and Card Shell

- `LauncherPanel`, `QueryField`, `ResultListView`, `ResultCellView`.
- Spotlight/Raycast-like AppKit visual language.
- Rounded card dashboard with edit buttons.
- Drag-to-reorder card position model.
- Keyboard navigation.
- Early fake static results, replaced by real module query dispatch.
- Snapshot diffing.
- Gate: fake keystroke -> paint p95 <= 30 ms.

## Phase 2: Core Data and Ranking

- Fuzzy matcher and ranker.
- Usage tracker.
- Local persistence. Current implementation uses JSON/UserDefaults/Keychain; SQLite remains optional, not active.
- Feature card layout persistence.
- Apps and Commands modules.
- First daily-dogfood build.

## Phase 3: Raycast-Like Workflows

- Translate module.
- Clipboard history module with secret filtering.
- Clipboard retention caps: 500 entries, 7 days, 100 KB per entry.
- Window Layouts module.
- Lazy Accessibility permission flow.

## Phase 4: Personal Knowledge and Secrets

- Secrets Vault with Keychain-backed encryption direction.
- Notes Graph module.
- Markdown vault tree.
- Typora open/edit integration.
- Backlink/tag graph index.

## Phase 5: Wordbook Migration

- Import `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- Preserve 9-stage review schedule.
- Review card with known/fuzzy/unknown actions.
- macOS speech for word/example.
- Daily goal/progress card.

## Phase 6: Polish and Release

- Settings.
- Multi-monitor/fullscreen Spaces verification.
- Developer ID signed and notarized DMG.
- Latency regression checklist.
- Local release script.

## Deferred

- Public plugin API.
- Cloud sync.
- Notes editor replacement.
- Complex animated graph rendering before the index is solid.
