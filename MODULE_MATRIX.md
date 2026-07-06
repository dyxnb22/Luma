# Module Matrix

## Scope

This file is the **Phase 2** product of the Luma stabilization investigation. It records the **current, as-built** set of Luma functional modules: business entrypoints, default state, data sources, permissions, cache/warmup, detail surfaces, core actions, hot-path participation, failure/diagnostic behavior, and current risks.

- This file does **not** propose refactors, does **not** fix bugs, does **not** change source code or tests, and does **not** rule on product priority.
- Where documentation (`docs/MODULES.md`, `docs/PERMISSIONS.md`, `docs/ENGINEERING.md`, `docs/DECISIONS.md`) and code disagree, both are recorded as "Doc says" / "Code fact" without a ruling.
- Where a fact could not be confirmed from the files read for this phase, it is marked "未确认" rather than guessed.
- File-path citations are included for every per-module conclusion.

## Inputs

Phase 0 / Phase 1 artifacts read directly:

- `/Users/diaoyuxuan/Luma/CURRENT_STATE.md`
- `/Users/diaoyuxuan/Luma/ARCHITECTURE_MAP.md`
- `/Users/diaoyuxuan/Luma/Package.swift`
- `/Users/diaoyuxuan/Luma/README.md`
- `/Users/diaoyuxuan/Luma/docs/ENGINEERING.md`
- `/Users/diaoyuxuan/Luma/docs/MODULES.md`
- `/Users/diaoyuxuan/Luma/docs/PERMISSIONS.md`
- `/Users/diaoyuxuan/Luma/docs/QA.md`
- `/Users/diaoyuxuan/Luma/docs/DECISIONS.md`

Key code entry points read directly:

- `Sources/LumaModules/BuiltInModules.swift`
- `Sources/LumaModules/ModuleRegistry.swift`
- `Sources/LumaModules/ModuleIdentifiers.swift`
- `Sources/LumaModules/ModuleSearchHints.swift`
- `Sources/LumaModules/FeatureCatalog.swift`
- `Sources/LumaCore/Modules/LumaModule.swift`
- `Sources/LumaCore/Modules/ModuleBundle.swift`
- `Sources/LumaCore/Modules/WarmupTier.swift`
- `Sources/LumaCore/Commands/CommandRouter.swift`
- `Sources/LumaApp/Composition/ModuleDetailRegistry.swift`
- `Sources/LumaApp/App/ModuleBootstrapper.swift`
- `Sources/LumaInfrastructure/Configuration/Configuration.swift`

Per-module source read directly (manifest + handle + warmup + teardown + actions):

- `Sources/LumaModules/Apps/AppsModule.swift`, `Apps/AppsModuleBundle.swift`
- `Sources/LumaModules/Clipboard/ClipboardModule.swift`, `Clipboard/ClipboardModuleBundle.swift`
- `Sources/LumaModules/Notes/NotesModule.swift`, `Notes/NotesModuleBundle.swift`
- `Sources/LumaModules/Todo/TodoModule.swift`, `Todo/TodoModuleBundle.swift`
- `Sources/LumaModules/Projects/ProjectsModule.swift`, `Projects/ProjectsModuleBundle.swift`
- `Sources/LumaModules/Quicklinks/QuicklinksModule.swift`, `Quicklinks/QuicklinksModuleBundle.swift`
- `Sources/LumaModules/Snippets/SnippetsModule.swift`, `Snippets/SnippetsModuleBundle.swift`
- `Sources/LumaModules/Translate/TranslateModule.swift`, `Translate/TranslateModuleBundle.swift`
- `Sources/LumaModules/Wordbook/WordbookModule.swift`, `Wordbook/WordbookModuleBundle.swift`
- `Sources/LumaModules/Media/MediaModule.swift`, `Media/MediaModuleBundle.swift`
- `Sources/LumaModules/Secrets/SecretsModule.swift`, `Secrets/SecretsModuleBundle.swift`
- `Sources/LumaModules/Commands/CommandsModule.swift`, `Commands/CommandsModuleBundle.swift`
- `Sources/LumaModules/WindowLayouts/WindowLayoutsModule.swift`, `WindowLayouts/WindowLayoutsModuleBundle.swift`
- `Sources/LumaModules/MenuItems/MenuItemsModule.swift`, `MenuItems/MenuItemsModuleBundle.swift`
- `Sources/LumaModules/KillProcess/KillProcessModule.swift`, `KillProcess/KillProcessModuleBundle.swift`
- `Sources/LumaModules/BrowserTabs/BrowserTabsModule.swift`, `BrowserTabs/BrowserTabsModuleBundle.swift`
- `Sources/LumaModules/Windows/WindowsModule.swift` (deferred; no bundle)
- `Sources/LumaModules/Workbench/WorkbenchCaptureEngine.swift`, `Workbench/WorkbenchCaptureDraftBuilder.swift` (Workbench is not a module; see Projects / Current Project section)

Test inventory read via directory listing: `Tests/LumaModulesTests/**/*.swift` (65 files).

## Classification Legend

### MVP Role

- **P0 主路径**: 用户认为软件可用必须稳定的功能。
- **P1 常用增强**: 主路径稳定后再保证的功能。
- **P2 专家/可选**: 可以默认关闭或延后恢复的功能。
- **Deferred**: 代码存在但默认不应进入主路径或需要特定条件。

This taxonomy is a product classification. Basis for each module's assignment is cited inline (manifest `defaultEnabled`, `docs/DECISIONS.md` D-012, `docs/MODULES.md` MVP list, and observed dependency weight). It is not a refactor recommendation.

### Permission

- None
- Accessibility
- Automation / AppleScript
- EventKit
- Keychain
- Pasteboard
- File System
- Translation framework
- Notification / Speech / AVFoundation
- Multiple

### Failure Behavior

- **Diagnostic required**: 失败必须显示 diagnostic/status row，不能静默空。
- **Empty acceptable**: 无数据时空结果可接受。
- **Permission banner**: 需要权限引导（permission row 或 panel banner）。
- **Degraded row**: 冷缓存/权限/超时要显示降级行。
- **Unknown**: 未确认。

### Hot Path

- **Yes**: `handle` 会被输入框高频调用，不能阻塞。
- **Partial**: 某些入口在 hot path，doctor/export/detail 不在。
- **No**: 只在显式动作、detail 或后台运行。

## Summary Matrix

Module identifiers from `Sources/LumaModules/ModuleIdentifiers.swift:4-20`. Manifest facts from each `*Module.swift` `manifest` definition. Trigger/alias/bareBehavior facts from each `*ModuleBundle.swift` `commands`. Warmup tier from each `*ModuleBundle.swift` `warmupTier`. Detail registration from `Sources/LumaApp/Composition/ModuleDetailRegistry.swift:77-131`.

| Module | Identifier | Triggers | Default State | MVP Role | Data Source | Permissions | Cache/Warmup | Detail View | Core Actions | Hot Path | Failure Behavior | Current Risks |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Apps | `luma.apps` | `app`, `apps` (alias); `app top` reserved | On (manifest `defaultEnabled: true`) | P0 | Installed apps index (`AppScanner.scan`), on-disk `AppIndexCache`, running bundle IDs via `RunningApplicationsClient`, process memory via `ProcessMemoryClient` | None for search; AX only via Open Apps window controls (lazy) | hotPath; warmup loads cache then refreshes from disk; `searchCache` LRU 32; running refresh loop 2s; memory-top TTL 2s | None registered | Launch/focus app, `app top` memory leaders, quit, reveal, copy path | Yes (global search contributing, fast tier) | Degraded row: `app top` cold → "Memory usage cache warming"; empty search acceptable | Disk-backed warmup async; `launcherFlowHarnessReplaysQuery` failure touches this path; running-app/AX lag |
| Clipboard | `luma.clipboard` | `clip`, `cb` (alias); bareBehavior `openDetail` | On | P0 | Pasteboard history (`clipboard-history.json` in `~/Library/Application Support/Luma/`), `ClipboardSnapshotService` polling | Accessibility (paste-directly); Pasteboard | hotPath; warmup loads store + starts 1s polling if `historyEnabled`; per-entry `cachedSearchHaystack`; `listQueryCache` | `ClipboardDetailView` | Copy entry, paste entry (AX), pin, delete, clear recent/today, source filter | Yes (global search contributing, deferred tier; ≥3 chars, capped 3) | Empty acceptable (open-detail row); paste → `permissionRequired` thrown when AX denied | Large `clipboard-history.json` (~38MB observed) + `.bak` quarantine; privacy filters; separate quarantine scheme from `JSONConfigPersistence` |
| Notes | `luma.notes` | `n`, `note`, `notes` (aliases); bareBehavior `openDetail` | On | P0 | Markdown files under configured root; `notes.json` config/cache; `NotesTreeIndex`/`NotesMetaIndex` in memory; FSEvents watch | File System (root folder); none for search | onDemand; warmup → `reloadFromConfig` builds tree/meta index + starts FSEvents watch | `NotesDetailView` | Open note (Typora/system), create note/folder, daily, capture, review week, doctor | Partial (targeted query uses warm in-memory index; detail/doctor off hot path) | Onboarding row "Choose a Notes root folder" when root unset (`NotesModule.swift:507-522`); empty search acceptable | Root must be configured; FSEvents async refresh staleness gated by `NotesDetailRefreshGate`; `notes.json` quarantined via `JSONConfigPersistence` |
| Todo | `luma.todo` | `t`, `todo` (alias); bareBehavior `openDetail` | On | P0 | EventKit reminders via `RemindersService`/`RemindersClient` | EventKit (Reminders) | hotPath; warmup caches authorization + due cache if authorized; `dueCacheRefreshTask`; EventKit store-change listener | `TodoDetailView` | List today/due, create reminder (NLP), complete/uncomplete | Partial (targeted query; bare `t` opens detail) | Permission row (`PermissionResultBuilder.row`) on `.denied`/`.notDetermined` (`TodoModule.swift:116-121,460-478`) | EventKit auth required; denied state must be actionable (D / QA); store-change listener |
| Projects | `luma.projects` | `p`, `proj`, `project` (aliases) | Off (manifest `defaultEnabled: false`) | P2 | `projects.json` (`ProjectStore`), `ProjectScanner.scan(roots:)` over configured roots, recent activity | File System (scan roots); none for search | onDemand; warmup → `refreshIndex` scans roots + builds `ProjectIndex` | `ProjectsDetailView` (manage) or `CurrentProjectDetailView` (default `proj` detail) | Open in Cursor/VSCode/Finder/Terminal, copy path, reveal, open notes, pin, aliases, add root | No (targeted + detail only; no background disk scan per query per `docs/MODULES.md:170`) | Onboarding row "Add a project scan root" when empty (`ProjectsModule.swift:252-273`) | Scan-root config required; `proj manage` vs current-project branching via `LauncherSharedState.pendingProjectsManage`; Workbench/CurrentProject surfaces attached here |
| Quicklinks | `luma.quicklinks` | `ql`, `quicklinks` (alias); bareBehavior `openDetail`; configured exact first-token triggers (e.g. `gh`, `g`, `swift`) | On | P0 | `quicklinks.json` (`QuicklinksStore`); in-memory `QuicklinksIndex` | None (opens URL via `WorkspaceService.openURL` http/https/mailto) | hotPath; warmup → `refreshCache` loads store + builds index | `QuicklinksDetailView` | Open URL (template expansion), copy URL, reveal config, add/edit/delete (confirm) | Yes (global search contributing, fast tier; exact-trigger match only) | Empty acceptable (manage row); `dataUnavailable` thrown on missing quicklink | URL scheme restricted (D-017); template variable engine shared with Snippets |
| Snippets | `luma.snippets` | `s`, `snip` (alias) | On | P0 | Snippet store (`SnippetsStore`/`JSONFileStore`); in-memory `cachedSnippets` | Accessibility (paste/insert); Pasteboard | hotPath; warmup loads `cachedSnippets` + binds pasteboard/AX/currentProject/selection clients | `SnippetsDetailView` | Copy snippet, paste/insert snippet (AX), create, duplicate, delete, exact-trigger expansion | Partial (targeted query + exact-trigger expansion in global search; excluded from `QuerySnapshotCache`) | `permissionRequired(.accessibility)` thrown on paste when AX denied (`SnippetsModule.swift:90-92`); copy still works | Excluded from query snapshot cache (`QuerySnapshotCache.swift:4-7`); `handle` must not await AX (test `snippetsHandleDoesNotAwaitAccessibility`) |
| Translate | `luma.translate` | `tr`, `translate` (alias); bareBehavior `openDetail` | On | P0 | Typed text in `perform`; system/Shortcut-backed `TranslationClient` | Translation framework (linked in `Package.swift:35`) | hotPath (tier); no cache; `handle` only builds a translate row, real work in `perform` | `TranslateDetailView` | Translate text, open detail with language chips | Partial (targeted query; bare `tr` opens detail) | Error mapping via `TranslationClient` (per `docs/MODULES.md:117`); failure tests exist | Real translation happens in `perform`/detail, not `handle`; target language from `ConfigurationStore.translationTargetLanguage` (default `en`) |
| Wordbook | `luma.wordbook` | `word`, `wb` (alias); bareBehavior `openDetail` | Off (manifest `defaultEnabled: false`) | P2 | Local SQLite via `WordbookStore`; in-memory `searchIndex` (up to 50k rows) + `searchResultCache` + due cache | None | hotPath (tier); warmup → `refreshDueCache(force)` + `reloadSearchIndex` + `WordbookStoreChangeHub` listener | `WordbookDetailView` | Search, `word review` start review, copy meaning | Partial (targeted query) | Degraded row "Loading due words…" when due cache cold (`WordbookModule.swift:128-143`); empty search acceptable | Default off; CSV import after enabling; large in-memory index; `perform` throws `unsupportedAction` (review runs in detail) |
| Media | `luma.media` (displayName "Records") | `rec`, `record`, `log`, `m`, `media` (aliases); bareBehavior `openDetail` | Off (manifest `defaultEnabled: false`) | P2 | `MediaStore` (`JSONFileStore`); in-memory `cachedItems` | None | onDemand; warmup loads `cachedItems` | `MediaDetailView` | Capture (DSL), search, edit, delete, copy summary, CSV export to `~/Downloads` | No (targeted + detail only) | Empty acceptable (manage-log row); `dataUnavailable` on missing item | Default off; CSV export writes to Downloads |
| Secrets | `luma.secrets` | `sec`, `secret`, `secrets` (aliases); bareBehavior `openDetail` | Off (manifest `defaultEnabled: false`) | P2 | Keychain (`KeychainSecretsStore`) + `secrets-metadata.json` (`SecretsVault`); values never in `handle` | Keychain | hotPath (tier); warmup configures auto-clear/relock + optionally unlocks (per `secretsRequireUnlockOnLaunch`) | `SecretsDetailView` | Unlock vault, copy secret (secure pasteboard with auto-clear), copy account, save/update/delete | Partial (targeted query; only labels, never values) | Locked row shown when vault locked (`SecretsModule.swift:50-52,126-142`); empty when unlocked+empty | Default off; excluded from `QuerySnapshotCache`; metadata JSON quarantined via `JSONConfigPersistence`; auto-clear timer from config |
| Commands | `luma.commands` | `settings`/`prefs`, `open-settings`, `reload-modules`, `exit`; `cmd` prefix; `doctor` | Off (manifest `defaultEnabled: false`) | P1 (doctor/export-diagnostics) / P2 (user scripts) | `commands.json` (`CommandsStore`) user scripts; built-in commands; `ConfigCorruptionRegistry`; `CrashLogBuffer` | None (scripts run via `ScriptRunnerService`; executable must live under `~/.luma/commands` per `docs/PERMISSIONS.md`) | onDemand; warmup reloads store + binds scriptRunner/currentProject/selection/reminders/menuBarTree/config | None registered | `settings`, `reload-modules`, `exit`/`quit`, `export-diagnostics`, `doctor`, run user script, reveal config | No (targeted only; not in global search contributing set) | `doctor` returns informational rows (`LumaDiagnostics.doctorRows`); script run records `CrashLogRecording` on failure | Default off means bare `exit`/`quit`/`doctor` do not respond until enabled (D-014); `doctor` is the global doctor surface; `export-diagnostics` writes `~/Library/Logs/Luma/diagnostics.json` |
| Window Layouts | `luma.window-layouts` | `win`, `wl`, `layout` (aliases) | Off (manifest `defaultEnabled: false`) | P2 | Focused window geometry via AX | Accessibility | hotPath (tier); warmup caches `isTrusted()` (TTL 5s); `requestAccess` invalidates cache | None registered | Apply layout preset (left/right/max/center halves/thirds), request AX, open System Settings | No (targeted only) | Permission row (`PermissionResultBuilder.row`) when AX denied (`WindowLayoutsModule.swift:71-93`) | Default off (D-020); AX-dependent; no warm-cache hot-path registration per D-020 |
| Menu Items | `luma.menu-items` | `mb`, `menu` (aliases) | Off (manifest `defaultEnabled: false`) | P2 | Cached AX menu tree via `MenuBarTreeService`; `menu-items.json` config (disabled bundle IDs) | Accessibility | onDemand; warmup loads config + `service.start`; `teardown` → `service.stop` | None registered | Search cached menu items, press menu item (AX) | Partial (targeted query; AX traversal in bounded cache refreshes, not per keystroke) | Diagnostic row on empty: `.degraded`/`.permissionRequired` per AX trust (`MenuItemsModule.swift:63-91`) | Default off; AX-dependent; stale cache; `menu-items.json` self-created on first load |
| Kill Process | `luma.kill-process` | `kill`, `quit`, `k` (aliases) | Off (manifest `defaultEnabled: false`) | P2 | Running GUI apps via `RunningProcessService`; cached `[RunningProcessRecord]` (TTL 3s) | None | hotPath (tier); warmup schedules refresh; `scheduleRefreshIfStale` | None registered | Normal quit, force kill (confirm), relaunch, `kill refresh` | No (targeted only) | Degraded row "Refreshing process list…" when cold cache refresh in flight (`KillProcessModule.swift:116-124`) | Default off; bare `quit`/`kill`/`k` resolve here (D-014); guarded bundle IDs require confirmation |
| Browser Tabs | `luma.browser-tabs` | `tab`, `tabs` (aliases) | Off (manifest `defaultEnabled: false`) | P2 | Cached Safari/Chrome/Brave/Edge/Arc tabs via `BrowserTabsService` + `AppleScriptRunner` | Automation (AppleScript) per browser | onDemand; warmup calls `service.searchableTabs()`; background refresh; `handle` uses `cachedTabs()` only | None registered | Activate tab, copy URL, save as Quicklink draft | No (targeted only; AppleScript must not run on hot path per `docs/MODULES.md:202`) | Diagnostic `.degraded` on no tabs / no match; surfaces `service.lastDiagnostic()` (`BrowserTabsModule.swift:38-65`) | Default off (Automation prompts sensitive); first cold query may return no rows while background refresh runs; test `browserTabsHandleUsesCacheOnlyPath` |
| Windows | `luma.windows` | (none registered) | Deferred — manifest `defaultEnabled: true` but **not** in `ModuleRegistry.allBundles` (`ModuleRegistry.swift:5-22`); only `BuiltInModules.makeDeferred()` (`BuiltInModules.swift:48-50`) | Deferred | `CGWindowListCopyWindowInfo` (on-screen windows) | Screen recording API (per `docs/PERMISSIONS.md:16`) | None — not registered for warmup | None registered | Focus window (`ActionKind.focusWindow`) | No (not registered) | Unknown — `handle` returns empty if no matches; no diagnostic | `handle` calls `CGWindowListCopyWindowInfo` directly (violates memory-only hot-path intent); deferred per `docs/MODULES.md:51` until warm-cache + tests land |
| Workbench / Current Project / Capture | (no module identifier; surfaces under `luma.projects`) | `proj` detail; cross-module capture actions | Not a module — capability surfaced via Projects detail + cross-module capture | P2 (Projects is default-off) | `CurrentProjectService` (LumaServices/Accessibility), `WorkbenchActivity` (`workbench-activity.json`), panel signals cache | Clipboard / selection / AX context | Capture engine lives in `LumaModules/Workbench` + `LumaApp/Composition` (`DefaultWorkbenchCaptureService`, `WorkbenchCaptureRunner`, `WorkbenchCommandExecutor`, `WorkbenchContextBuilder`) | `CurrentProjectDetailView` (default `proj` detail, `ModuleDetailRegistry.swift:121-126`) | Convert selection/clipboard/context → note/todo/snippet/quicklink/project link; workspace row actions | No | Diagnostic summary via `WorkbenchDiagnosticSummaryTests` | Not a standalone module; `proj` preview must not stall on selection fetch unless attach/capture (`docs/QA.md:139`); `WorkbenchActivity` quarantined via `JSONConfigPersistence` |

Notes on the summary table:

- "Default State" reflects the manifest `defaultEnabled` value read from each `*Module.swift`, cross-checked against `ModuleWarmupDefaults.defaultEnabledModuleIDs`/`expertDefaultOffModuleIDs` in `Sources/LumaCore/Modules/WarmupTier.swift:32-57`. Runtime enablement is overridden by `ConfigurationStore.enabledModules()` (UserDefaults) and migrated by `migrateIfNeeded()` to schema v2 (`Configuration.swift:197-216`).
- "Detail View" reflects only `ModuleDetailRegistry.makeDefault()` registrations (`ModuleDetailRegistry.swift:77-131`). Apps, Commands, Menu Items, Kill Process, Browser Tabs, Window Layouts, and Windows have **no** registered in-panel detail view.
- "Hot Path" reflects `GlobalSearchTiers.contributingModuleIDs` (apps, quicklinks, clipboard) for global search fan-out plus each module's `warmupTier` and whether `handle` is invoked on typed queries. See "Hot Path And Blocking Risk" below.

## MVP Main Path Candidates

Basis: `docs/DECISIONS.md` D-012 (seven default-on MVP modules), `docs/MODULES.md:18` MVP default-on list, `docs/QA.md` core manual smoke, and the manifest `defaultEnabled` values read above. This is product classification, not a code change plan.

- **Launcher 唤起/隐藏** — not a module, but the前置 capability for every module entry. Recorded here because all module surfaces depend on hotkey show/hide + panel visibility session. Code: `Sources/LumaApp/App/Hotkey/*`, `LauncherWindowController.swift`. Phase 0 fact: hotkey p95 ≈ 8.3s (`CURRENT_STATE.md`).
- **Apps 搜索和打开** — `luma.apps`, default-on, global search fast tier, primary "open/activate app" action. (`AppsModuleBundle.swift`, `AppsModule.swift:125-132`)
- **Clipboard 基本搜索/复制** — `luma.clipboard`, default-on, global search contributing (deferred tier), primary copy + paste-directly. (`ClipboardModuleBundle.swift`, `ClipboardModule.swift:80-100`)
- **Notes 基本打开/创建** — `luma.notes`, default-on, bare `n` opens detail, `n new`/`n daily` create. (`NotesModuleBundle.swift`, `NotesModule.swift:58-83`)
- **Todo 基本列表/捕获** — `luma.todo`, default-on, EventKit pass-through. (`TodoModuleBundle.swift`, `TodoModule.swift:106-124`)
- **Translate 基本翻译** — `luma.translate`, default-on, `tr <text>` translates. (`TranslateModuleBundle.swift`, `TranslateModule.swift:17-45`)
- **Snippets 复制/展开** — `luma.snippets`, default-on, exact-trigger expansion + copy. (`SnippetsModuleBundle.swift`, `SnippetsModule.swift:34-65`)
- **Quicklinks URL 启动** — `luma.quicklinks`, default-on, exact first-token triggers. (`QuicklinksModuleBundle.swift`, `QuicklinksModule.swift:27-39`)
- **Settings / Commands doctor / export-diagnostics** — `luma.commands` is default-off, but `settings`/`open-settings` is the documented primary entry to preferences and `cmd doctor` / `cmd export-diagnostics` are the diagnostics surfaces (`docs/ENGINEERING.md:180`, `CommandsModule.swift:188-247`). Recorded as P1 because the diagnostics/permissions trust surface depends on them even though the module is default-off.
- **Permissions guidance / diagnostic surfacing** — cross-cutting: `PermissionBannerController`, `AccessibilityGuidancePolicy`, `PermissionResultBuilder.row` (used by Todo, Window Layouts), `ModuleDiagnosticResults.informationalRow` (used by Kill Process), `.degraded`/`.permissionRequired` diagnostics (Menu Items, Browser Tabs, Apps memory-top). Not a module; recorded as a P0 trust surface.

## Default-Off / Deferred Candidates

From manifest `defaultEnabled: false` and `ModuleWarmupDefaults.expertDefaultOffModuleIDs` (`WarmupTier.swift:47-57`):

- **Commands** (`luma.commands`) — default-off; expert scripts + built-ins. (`CommandsModuleBundle.swift:6-8`)
- **Media / Records** (`luma.media`) — default-off; personal logbook. (`MediaModuleBundle.swift:6-8`)
- **Browser Tabs** (`luma.browser-tabs`) — default-off; Automation prompts sensitive. (`BrowserTabsModuleBundle.swift:6-8`)
- **Menu Items** (`luma.menu-items`) — default-off; AX-dependent. (`MenuItemsModuleBundle.swift:6-8`)
- **Window Layouts** (`luma.window-layouts`) — default-off (D-020); AX-dependent. (`WindowLayoutsModuleBundle.swift:6-8`)
- **Wordbook** (`luma.wordbook`) — default-off; CSV import after enabling. (`WordbookModuleBundle.swift:6-8`)
- **Secrets** (`luma.secrets`) — default-off; Keychain vault. (`SecretsModuleBundle.swift:6-8`)
- **Kill Process** (`luma.kill-process`) — default-off; expert quit/force-kill. (`KillProcessModuleBundle.swift:6-8`)
- **Projects** (`luma.projects`) — default-off; scan roots must be configured. (`ProjectsModuleBundle.swift:6-8`)

Deferred (source-retained, not registered):

- **Windows** (`luma.windows`) — `BuiltInModules.makeDeferred()` returns `[WindowsModule()]` only (`BuiltInModules.swift:48-50`); absent from `ModuleRegistry.allBundles` (`ModuleRegistry.swift:5-22`). `docs/MODULES.md:51` says `handle()` must not ship on the hot path until warm-cache + tests land.

## macOS Permission Dependency Map

| Permission / System Capability | Modules / Features | Failure UI | Source Files |
| --- | --- | --- | --- |
| Accessibility / AX | Apps (Open Apps window controls, lazy), Snippets (paste/insert), Window Layouts (move window), Menu Items (menu tree), Windows (deferred), Clipboard (paste-directly) | Permission banner (`PermissionBannerController`/`AccessibilityGuidancePolicy`); `PermissionResultBuilder.row` (Window Layouts); `ModuleError.permissionRequired(.accessibility)` thrown (Snippets, Clipboard paste); AX-dependent module set `BuiltInModules.accessibilityDependentModuleIDs = {.windows, .snippets, .windowLayouts, .menuItems}` | `Sources/LumaServices/Accessibility/AXService.swift`, `Sources/LumaApp/Launcher/PermissionBannerController.swift`, `Sources/LumaModules/BuiltInModules.swift:52`, `Sources/LumaModules/WindowLayouts/WindowLayoutsModule.swift:71-93`, `Sources/LumaModules/Snippets/SnippetsModule.swift:90-92`, `Sources/LumaModules/Clipboard/ClipboardModule.swift:166` |
| Automation / AppleScript | Browser Tabs (Safari/Chrome/Brave/Edge/Arc tabs) | Diagnostic `.degraded` / `service.lastDiagnostic()` (`BrowserTabsModule.swift:38-65`); actionable denial per `docs/QA.md:148` | `Sources/LumaServices/BrowserTabs/AppleScriptRunner.swift`, `BrowserTabsService.swift`, `SafariAdapter.swift`, `ChromiumAdapter.swift`, `Sources/LumaModules/BrowserTabs/BrowserTabsModule.swift` |
| EventKit / Reminders | Todo | `PermissionResultBuilder.row` on `.denied`/`.notDetermined` (`TodoModule.swift:116-121,460-478`); actionable denial per `docs/QA.md:149` | `Sources/LumaServices/EventKit/RemindersService.swift`, `Sources/LumaModules/Todo/TodoModule.swift` |
| Keychain | Secrets | Locked vault row (`SecretsModule.swift:50-52,126-142`); unlock flow in detail | `Sources/LumaModules/Secrets/KeychainSecretsStore.swift`, `SecretsVault.swift`, `Sources/LumaModules/Secrets/SecretsModule.swift` |
| Pasteboard | Clipboard (read/write), Snippets (copy), Secrets (secure write + auto-clear), Quicklinks (copy URL), Commands (template var) | `permissionRequired` only on paste-directly (AX); copy always works | `Sources/LumaServices/Pasteboard/PasteboardService.swift`, `ClipboardSnapshotService.swift`, `Sources/LumaModules/Clipboard/ClipboardModule.swift`, `Sources/LumaModules/Snippets/SnippetsModule.swift` |
| File System / FSEvents | Notes (root watch), Projects (scan roots), Commands (script exec under `~/.luma/commands`), Menu Items (`menu-items.json` config), Clipboard (`clipboard-history.json`), all `JSONConfigPersistence`/`JSONFileStore` users | Onboarding rows (Notes "Choose a Notes root folder", Projects "Add a project scan root"); corrupt-file quarantine + `cmd doctor` listing | `Sources/LumaServices/FileSystem/FSEventsService.swift`, `Sources/LumaCore/Persistence/JSONConfigPersistence.swift`, `JSONFileStore.swift`, `Sources/LumaModules/Notes/NotesModule.swift:507-522`, `Sources/LumaModules/Projects/ProjectsModule.swift:252-273` |
| Translation framework | Translate | Mapped errors from `TranslationClient` (per `docs/MODULES.md:117`); `TranslationFailureTests` exist | `Sources/LumaServices/Translation/*`, `Sources/LumaModules/Translate/TranslateModule.swift`, `Package.swift:35` |
| Notification / Speech / AVFoundation | Linked in `Package.swift:37-38` (`AVFoundation`, `UserNotifications`) for `LumaServices` | 未确认 — no module in this matrix was confirmed to surface these to users from the files read | `Package.swift:34-39`, `Sources/LumaServices/Process/NotificationService.swift`, `Sources/LumaServices/Speech/*` |
| Screen recording API | Windows (deferred) | None — module not registered | `Sources/LumaModules/Windows/WindowsModule.swift`, `docs/PERMISSIONS.md:16` |

## Diagnostic Requirements

Scenes that must **not** fail silently, based on observed diagnostic/permission-row code and `docs/ENGINEERING.md` ("disabled or permission-blocked modules return diagnostic rows, not silent empty results"):

- **权限拒绝**
  - Todo EventKit denied/notDetermined → permission row (`TodoModule.swift:116-121,460-478`).
  - Window Layouts AX denied → permission row (`WindowLayoutsModule.swift:71-93`).
  - Snippets/Clipboard paste-directly AX denied → `ModuleError.permissionRequired(.accessibility)` (`SnippetsModule.swift:90-92`, `ClipboardModule.swift:143-145,166`).
  - Menu Items AX untrusted → `.permissionRequired` diagnostic (`MenuItemsModule.swift:76-81`).
  - Browser Tabs Automation denied/timeout → `service.lastDiagnostic()` / `.degraded` (`BrowserTabsModule.swift:38-65`).
- **冷缓存 warming**
  - Apps `app top` cold → "Memory usage cache warming" degraded row (`AppsModule.swift:184-188`).
  - Kill Process cold refresh in flight → "Refreshing process list…" informational row (`KillProcessModule.swift:116-124`).
  - Wordbook due cache cold → "Loading due words…" informational row (`WordbookModule.swift:128-143`).
  - Menu Items empty cache → `.degraded`/`.permissionRequired` (`MenuItemsModule.swift:69-86`).
- **timeout** — `QueryDispatcher` returns `ModuleResult.empty(for:diagnostic: ModuleDiagnostic(kind: .timeout, message: "Module timed out"))` on `manifest.queryTimeout` exceeded (`ARCHITECTURE_MAP.md:605`; `Sources/LumaCore/Modules/Timeout.swift`).
- **数据源缺失**
  - Notes root unset → onboarding row "Choose a Notes root folder" (`NotesModule.swift:507-522`).
  - Projects no roots/matches → onboarding row "Add a project scan root" (`ProjectsModule.swift:252-273`).
  - Browser Tabs no tabs → `.degraded` diagnostic (`BrowserTabsModule.swift:42-48`).
- **配置损坏** — `JSONConfigPersistence.load` quarantines + records `ConfigCorruptionRegistry`; `cmd doctor` lists corrupt files (`CommandsModule.swift:200-203,219`; `ARCHITECTURE_MAP.md:847-849`).
- **store 读写失败** — `ClipboardHistoryStore` has its own quarantine (`ClipboardHistoryStore.swift:477-483`, `.corrupt-<ts>.bak`); `JSONFileStore` has its own quarantine path that does **not** call `ConfigCorruptionRegistry` (`ARCHITECTURE_MAP.md:847`).
- **external app/system API 不可用** — platform actions (paste/focus/insert/open URL/window layout) must propagate errors from platform clients and must not report success when the platform call no-ops (`docs/ENGINEERING.md` per `CURRENT_STATE.md:139`).

Scenes where **empty is acceptable** (no diagnostic required by current code): Clipboard no matches (open-detail row, `ClipboardModule.swift:102-114`), Snippets no matches (`SnippetsModule.swift:56-61`), Media no matches (`MediaModule.swift:54-55`), Translate no payload (empty), Quicklinks no exact-trigger match (`QuicklinksModule.swift:35-37`).

## Hot Path And Blocking Risk

Facts from `ARCHITECTURE_MAP.md` (Phase 1) cross-checked against code read this phase:

- **`handle` memory-only** is a documentation constraint (`docs/ENGINEERING.md:113`), **not** a type-system or static-analysis enforcement. `QueryContext` carries only a `deadline` + `QueryPlatformClients` (`Sources/LumaCore/Modules/ModuleContext.swift:65-75`); it does not sandbox module code. `scripts/scan_appkit_executor_risk.sh` targets AppKit executor boundaries, not disk/network access in `handle`. Targeted tests (`ModuleHandleContractTests.swift`: `snippetsHandleDoesNotAwaitAccessibility`, `browserTabsHandleUsesCacheOnlyPath`) act as partial proxies. (`ARCHITECTURE_MAP.md:631-638`)
- **Global search fan-out** is narrowed in production to `GlobalSearchTiers.contributingModuleIDs` = **apps, quicklinks, clipboard** (`Sources/LumaCore/Modules/WarmupTier.swift:69-73`; applied via `ModuleBootstrapper.swift:26` → `host.configureGlobalSearchModuleIDs(ModuleRegistry.globalSearchModuleIDs)`).
  - Fast tier (run first): `GlobalSearchTiers.fastModuleIDs` = apps, quicklinks, snippets — intersected with contributing = **apps, quicklinks** (`ModuleRegistry.swift:66-68`).
  - Deferred tier (~1 ms delay): contributing − fast = **clipboard** (`ModuleRegistry.swift:71-73`; `QueryDispatcher.swift:73-77,117-120`).
  - Snippets is fast-tier but **not** a contributing module for global search; it participates only via exact-trigger expansion (handled outside the contributing fan-out).
- **Targeted query** routes to a single module via `dispatchTargeted` (`LauncherViewModel.swift:57,95`); every registered module's `handle` can be hit on its own trigger, so each module's `handle` is on its own hot path even if not in global fan-out.
- **handle should be read-only memory** per docs. Observed implementations:
  - Apps `handle` reads `cachedSearch`/`cachedRunningBundleIDs` only; cold memory-top schedules background refresh (`AppsModule.swift:125-131,174-188`).
  - Clipboard `handle` calls `store.search` only; global search gated to ≥3 chars and capped at 3 (`ClipboardModule.swift:80-99`).
  - Quicklinks `handle` matches in-memory `index` only (`QuicklinksModule.swift:27-39`).
  - Browser Tabs `handle` calls `service.cachedTabs()` only (test-enforced) (`BrowserTabsModule.swift:32`).
  - **Windows `handle` calls `CGWindowListCopyWindowInfo` directly** (`WindowsModule.swift:18,52-68`) — this is the documented memory-only violation; the module is deferred and not registered, so it does not run in production fan-out today.
- **warmup/cache existence**: confirmed for Apps, Clipboard, Notes, Todo, Wordbook, Quicklinks, Snippets, KillProcess, MenuItems, BrowserTabs (see per-module notes). No generic enforcement mechanism ensures every module maintains a warm cache; the constraint is module-by-module.
- **query timeout** comes from each `manifest.queryTimeout` (per-module values listed in per-module notes); enforced by `Timeout.run` in `QueryDispatcher` (`ARCHITECTURE_MAP.md:605`).

## Per-Module Detail Notes

### Apps

- Identifier: `luma.apps` (`ModuleIdentifiers.swift:4`)
- Manifest/default: `defaultEnabled: true`, priority 5, `queryTimeout: 40ms`, capabilities `[queryable, providesActions, backgroundUpdater]` (`AppsModule.swift:5-12`); warmupTier `.hotPath` (`AppsModuleBundle.swift:5`)
- Triggers: `app` (primary), `apps` (alias); `bareBehavior: .globalSearchShadow`, `bareReservedPayloads: ["top", "?", "help"]` (`AppsModuleBundle.swift:10-32`). Single-character `app` would route via `CommandRouter` to targeted (trigger match) for bare, or globalSearch for `app <query>` because of shadow behavior (`CommandRouter.swift:37-41`).
- Data source: `AppScanner.scan()` disk index, on-disk `AppIndexCache`, `RunningApplicationsClient`, `ProcessMemoryClient` (`AppsModule.swift:30-51,95-107`)
- Permissions: None for search; AX only via Open Apps window controls (lazy per D-010) — Apps is **not** in `BuiltInModules.accessibilityDependentModuleIDs` (`BuiltInModules.swift:52`)
- Cache/warmup: warmup loads cache then refreshes from disk; `searchCache` LRU 32; running refresh loop every 2s; memory-top TTL 2s; `teardown` cancels all tasks (`AppsModule.swift:30-62,109-123`)
- Detail view: None registered in `ModuleDetailRegistry.makeDefault()`
- Core actions: launch/activate app, `app top` memory leaders + quit, reveal in Finder, copy path, login items settings, quit running app (`AppsModule.swift:199-256`)
- Hot path: Yes — global search fast tier + targeted
- Failure/diagnostic behavior: `app top` cold → `.degraded` "Memory usage cache warming" (`AppsModule.swift:184-188`); empty search acceptable
- Tests: `AppsModuleTests.swift`, `AppsModuleTopQueryPerformanceTests.swift`, `AppsMemoryTopSWRTests.swift`, `AppIndexTests.swift`, `PinyinIndexTests.swift`, `ModuleHandleContractTests.swift`, `ModuleColdCacheTests.swift`, `BuiltInModulesTests.swift`
- Current risks: disk-backed warmup is async; `launcherFlowHarnessReplaysQuery` (Phase 0 failure) exercises this path; running-app indicator depends on `RunningApplicationsClient` timing
- Sources: `Sources/LumaModules/Apps/AppsModule.swift`, `Apps/AppsModuleBundle.swift`

### Clipboard

- Identifier: `luma.clipboard` (`ModuleIdentifiers.swift:6`)
- Manifest/default: `defaultEnabled: true`, priority 1, `queryTimeout: 30ms`, `[queryable, providesActions, backgroundUpdater]` (`ClipboardModule.swift:5-12`); warmupTier `.hotPath`
- Triggers: `clip` (primary), `cb` (alias); `bareBehavior: .openDetail` (`ClipboardModuleBundle.swift:23-44`)
- Data source: `clipboard-history.json` in `~/Library/Application Support/Luma/` via `ClipboardHistoryStore`; `ClipboardSnapshotService` polling (`ClipboardModule.swift:30-37,244-304`)
- Permissions: Pasteboard (read/write); Accessibility (paste-directly insert)
- Cache/warmup: warmup loads retention/capture policy + starts 1s polling if `historyEnabled`; per-entry `cachedSearchHaystack`; `listQueryCache`; `teardown` cancels polling (`ClipboardModule.swift:54-78,244-252`)
- Detail view: `ClipboardDetailView` (`ModuleDetailRegistry.swift:91-97`)
- Core actions: copy entry, paste entry (AX), pin/unpin, delete, clear recent/today, source filter, cross-module (append to note, create snippet) (`ClipboardModule.swift:132-149,306-369`)
- Hot path: Yes — global search contributing (deferred tier), ≥3 chars, capped at 3 (`ClipboardModule.swift:93-99`)
- Failure/diagnostic behavior: empty → open-detail row; paste-directly → `ModuleError.permissionRequired(.accessibility)` when AX denied (`ClipboardModule.swift:143-145,166`)
- Tests: `ClipboardPersistenceTests.swift`, `ClipboardHistoryTests.swift`, `ClipboardSearchPerformanceTests.swift`, `PasteOutcomeTests.swift`
- Current risks: `clipboard-history.json` observed ~38MB with a `.corrupt-<ts>.bak` sibling (`ARCHITECTURE_MAP.md:842-843`); separate quarantine scheme from `JSONConfigPersistence` (`ClipboardHistoryStore.swift:477-483`); privacy filters must skip secret-looking values
- Sources: `Sources/LumaModules/Clipboard/ClipboardModule.swift`, `Clipboard/ClipboardModuleBundle.swift`, `Clipboard/ClipboardHistoryStore.swift`

### Notes

- Identifier: `luma.notes` (`ModuleIdentifiers.swift:12`)
- Manifest/default: `defaultEnabled: true`, priority 2, `queryTimeout: 40ms`, `[queryable, providesActions, backgroundUpdater]` (`NotesModule.swift:11-17`); warmupTier `.onDemand` (`NotesModuleBundle.swift:5`)
- Triggers: `n` (primary), `note`/`notes` (aliases); `bareBehavior: .openDetail` (`NotesModuleBundle.swift:23-48`)
- Data source: Markdown files under configured root; `notes.json` config/cache; `NotesTreeIndex`/`NotesMetaIndex` in memory; FSEvents watch (`NotesModule.swift:44-46,275-292`)
- Permissions: File System (root folder); none for search
- Cache/warmup: warmup → `reloadFromConfig` builds tree/meta index + starts FSEvents watch; `teardown` cancels watch (`NotesModule.swift:44-56`)
- Detail view: `NotesDetailView` (`ModuleDetailRegistry.swift:98-100`)
- Core actions: open note (Typora/system editor via `openLocalFileURL` after containment, D-016), create note/folder, daily, `n cap`, `n review week`, `n doctor` (off hot path) (`NotesModuleBundle.swift:36-44`; `docs/MODULES.md:96-102`)
- Hot path: Partial — targeted query uses warm in-memory index; detail/doctor off hot path
- Failure/diagnostic behavior: onboarding row "Choose a Notes root folder" when root unset (`NotesModule.swift:507-522`); empty search acceptable
- Tests: `NotesMetaTests.swift`, `NotesOpenPathSecurityTests.swift`, `NotesCaptureTests.swift`, `NotesCreateOpensLocalFileTests.swift`, `NotesTreeIndexTests.swift`, `NotesPortabilityTests.swift`, `NotesImageToolsTests.swift`
- Current risks: root must be configured; FSEvents async refresh staleness gated by `NotesDetailRefreshGate` (`ARCHITECTURE_MAP.md:764`); `notes.json` quarantined via `JSONConfigPersistence`
- Sources: `Sources/LumaModules/Notes/NotesModule.swift`, `Notes/NotesModuleBundle.swift`

### Todo

- Identifier: `luma.todo` (`ModuleIdentifiers.swift:8`)
- Manifest/default: `defaultEnabled: true`, priority 3, `queryTimeout: 60ms`, `[queryable, providesActions]` (`TodoModule.swift:42-48`); warmupTier `.hotPath` (`TodoModuleBundle.swift:5`)
- Triggers: `t` (primary), `todo` (alias); `bareBehavior: .openDetail` (`TodoModuleBundle.swift:23-46`)
- Data source: EventKit reminders via `RemindersService`/`RemindersClient` (`TodoModule.swift:85-92`)
- Permissions: EventKit (Reminders)
- Cache/warmup: warmup caches `authorization` + `refreshDueCache(force)` if authorized; `dueCacheRefreshTask`; EventKit store-change listener `storeChangesTask`; `teardown` cancels both (`TodoModule.swift:85-99,305-315`)
- Detail view: `TodoDetailView` (`ModuleDetailRegistry.swift:110-112`)
- Core actions: list today/due, create reminder (NLP via `TodoTimeParser`), complete/uncomplete, permission request/grant (`TodoModule.swift:106-124,460-478`)
- Hot path: Partial — targeted query; bare `t` opens detail
- Failure/diagnostic behavior: permission row on `.denied`/`.notDetermined` (`TodoModule.swift:116-121,460-478`); `accessDenied` flag set for `.denied`
- Tests: `TodoTimeParserTests.swift`, `TodoModuleStoreChangesTests.swift`
- Current risks: EventKit auth required; denied state must be actionable (`docs/QA.md:149`); store-change listener lifecycle
- Sources: `Sources/LumaModules/Todo/TodoModule.swift`, `Todo/TodoModuleBundle.swift`

### Projects

- Identifier: `luma.projects` (`ModuleIdentifiers.swift:16`)
- Manifest/default: `defaultEnabled: false`, priority 4, `queryTimeout: 30ms`, `[queryable, providesActions, backgroundUpdater]` (`ProjectsModule.swift:5-12`); warmupTier `.onDemand` (`ProjectsModuleBundle.swift:5`); `defaultOffNote` present (`ProjectsModuleBundle.swift:6-8`)
- Triggers: `p` (primary), `proj`/`project` (aliases) (`ProjectsModuleBundle.swift:13-34`)
- Data source: `projects.json` (`ProjectStore`), `ProjectScanner.scan(roots:)`, recent activity (`ProjectsModule.swift:145-149`)
- Permissions: File System (scan roots); none for search
- Cache/warmup: warmup → `refreshIndex` scans roots + builds `ProjectIndex`; `refreshIndex` re-runs after mutations (`ProjectsModule.swift:22-24,145-149`)
- Detail view: `ProjectsDetailView` (manage mode) **or** `CurrentProjectDetailView` (default `proj` detail), branched on `LauncherSharedState.pendingProjectsManage` (`ModuleDetailRegistry.swift:116-127`)
- Core actions: open in Cursor/VSCode/Finder/Terminal, copy path, reveal, open notes for project, pin, aliases, opener, add root, add manual project (`ProjectsModule.swift:55-123`)
- Hot path: No — targeted + detail only; no background disk scan per query (`docs/MODULES.md:170`)
- Failure/diagnostic behavior: onboarding row "Add a project scan root" / "No matching projects" (`ProjectsModule.swift:252-273`)
- Tests: `ProjectsModuleTests.swift`, `ProjectContextSuggestionsTests.swift`, `CurrentProjectFixesTests.swift`
- Current risks: scan-root config required; Workbench/CurrentProject surfaces attached here (see Workbench section); `proj manage` vs current-project branching via shared mutable state
- Sources: `Sources/LumaModules/Projects/ProjectsModule.swift`, `Projects/ProjectsModuleBundle.swift`

### Quicklinks

- Identifier: `luma.quicklinks` (`ModuleIdentifiers.swift:17`)
- Manifest/default: `defaultEnabled: true`, priority 5, `queryTimeout: 15ms`, `[queryable, providesActions]` (`QuicklinksModule.swift:5-13`); warmupTier `.hotPath`
- Triggers: `ql` (primary), `quicklinks` (alias); `bareBehavior: .openDetail`; configured exact first-token triggers (e.g. `gh`, `g`, `swift`) (`QuicklinksModuleBundle.swift:23-46`)
- Data source: `quicklinks.json` (`QuicklinksStore`); in-memory `QuicklinksIndex` + `cachedQuicklinks` (`QuicklinksModule.swift:15-17,125-128`)
- Permissions: None (opens URL via `WorkspaceService.openURL`, http/https/mailto only — D-017)
- Cache/warmup: warmup → `refreshCache` loads store + builds index (`QuicklinksModule.swift:23-25,125-128`)
- Detail view: `QuicklinksDetailView` (`ModuleDetailRegistry.swift:128-130`)
- Core actions: open URL (template expansion via `QuicklinkTemplateRenderer`), copy URL, reveal config, add/edit/delete (confirm) (`QuicklinksModule.swift:41-68,78-101`)
- Hot path: Yes — global search fast tier; exact-trigger match only (no fuzzy global) (`QuicklinksModule.swift:35-39`)
- Failure/diagnostic behavior: empty → manage row; `ModuleError.dataUnavailable` on missing quicklink (`QuicklinksModule.swift:48-51,59-62`)
- Tests: `QuicklinksModuleTests.swift`, `QuicklinksTests.swift`
- Current risks: URL scheme restricted (D-017); template variable engine shared with Snippets; persistent deletes require confirmation (`docs/MODULES.md:178`)
- Sources: `Sources/LumaModules/Quicklinks/QuicklinksModule.swift`, `Quicklinks/QuicklinksModuleBundle.swift`

### Snippets

- Identifier: `luma.snippets` (`ModuleIdentifiers.swift:14`)
- Manifest/default: `defaultEnabled: true`, priority 2, `queryTimeout: 20ms`, `[queryable, providesActions]` (`SnippetsModule.swift:5-13`); warmupTier `.hotPath`
- Triggers: `s` (primary), `snip` (alias); no `bareBehavior` set; `isBareOpenDetailReturn` special-cases `s new`/`s new <title>` to open detail (`CommandRouter.swift:65-68`)
- Data source: `SnippetsStore` (`JSONFileStore`); in-memory `cachedSnippets` (`SnippetsModule.swift:15-16,181-183`)
- Permissions: Accessibility (paste/insert); Pasteboard
- Cache/warmup: warmup loads `cachedSnippets` + binds pasteboard/AX/currentProject/selection clients (`SnippetsModule.swift:26-32`)
- Detail view: `SnippetsDetailView` (`ModuleDetailRegistry.swift:101-103`)
- Core actions: copy snippet, paste/insert snippet (AX), create, duplicate, delete, mark used, exact-trigger expansion via `snippetForTrigger` (`SnippetsModule.swift:67-99,146-165`)
- Hot path: Partial — targeted query + exact-trigger expansion in global search; excluded from `QuerySnapshotCache` (`QuerySnapshotCache.swift:4-7`)
- Failure/diagnostic behavior: `permissionRequired(.accessibility)` thrown on paste when AX denied; copy still works (`SnippetsModule.swift:90-92,155-157`); empty library → empty-library row (`SnippetsModule.swift:216-231`)
- Tests: `SnippetsStoreTests.swift`, `SnippetIndexTests.swift`, `ModuleHandleContractTests.swift` (`snippetsHandleDoesNotAwaitAccessibility`)
- Current risks: excluded from query snapshot cache; `handle` must not await AX (test-enforced); variable expansion touches clipboard/selection/project context in `perform`
- Sources: `Sources/LumaModules/Snippets/SnippetsModule.swift`, `Snippets/SnippetsModuleBundle.swift`

### Translate

- Identifier: `luma.translate` (`ModuleIdentifiers.swift:9`)
- Manifest/default: `defaultEnabled: true`, priority 3, `queryTimeout: 60ms`, `[queryable, providesActions]` (`TranslateModule.swift:5-13`); warmupTier `.hotPath`
- Triggers: `tr` (primary), `translate` (alias); `bareBehavior: .openDetail` (`TranslateModuleBundle.swift:23-44`)
- Data source: typed text in `perform`; system/Shortcut-backed `TranslationClient` (`TranslateModule.swift:17-45`)
- Permissions: Translation framework (linked `Package.swift:35`)
- Cache/warmup: hotPath tier; no cache; `handle` only builds a translate row, real work in `perform`/detail
- Detail view: `TranslateDetailView` (`ModuleDetailRegistry.swift:79-90`)
- Core actions: translate text, open detail with language chips, open Translation Settings (`TranslateModuleBundle.swift:36-41`; `ModuleDetailRegistry.swift:79-90`)
- Hot path: Partial — targeted query; bare `tr` opens detail
- Failure/diagnostic behavior: error mapping via `TranslationClient` (`docs/MODULES.md:117`); `TranslationFailureTests` exist
- Tests: `TranslationFailureTests.swift`, `TranslateBareContractTests.swift`
- Current risks: real translation happens in `perform`/detail, not `handle`; target language from `ConfigurationStore.translationTargetLanguage` (default `en`, `Configuration.swift:90-92`)
- Sources: `Sources/LumaModules/Translate/TranslateModule.swift`, `Translate/TranslateModuleBundle.swift`

### Wordbook

- Identifier: `luma.wordbook` (`ModuleIdentifiers.swift:13`)
- Manifest/default: `defaultEnabled: false`, priority 2, `queryTimeout: 40ms`, `[queryable, providesActions, backgroundUpdater]` (`WordbookModule.swift:5-12`); warmupTier `.hotPath`; `defaultOffNote` present (`WordbookModuleBundle.swift:6-8`)
- Triggers: `word` (primary), `wb` (alias); `bareBehavior: .openDetail`; `word review` special-cased in `isBareOpenDetailReturn` (`CommandRouter.swift:72-74`)
- Data source: local SQLite via `WordbookStore`; in-memory `searchIndex` (up to 50k rows), `searchResultCache`, due cache (`WordbookModule.swift:14-19,245-261`)
- Permissions: None
- Cache/warmup: warmup → `refreshDueCache(force)` + `reloadSearchIndex` + `WordbookStoreChangeHub` listener; `teardown` cancels both tasks (`WordbookModule.swift:37-55`)
- Detail view: `WordbookDetailView` (`ModuleDetailRegistry.swift:113-115`)
- Core actions: search, `word review` start review (opens detail), copy meaning; `perform` throws `unsupportedAction` (review runs in detail) (`WordbookModule.swift:62-103,168-191`)
- Hot path: Partial — targeted query
- Failure/diagnostic behavior: degraded "Loading due words…" row when due cache cold (`WordbookModule.swift:128-143`); empty search acceptable; `moduleHandleCold` counter incremented
- Tests: `WordbookModuleTests.swift`, `WordbookCSVImporterTests.swift`, `WordbookStoreMasteredTests.swift`, `WordbookStoreDailyStatsTests.swift`, `WordbookSessionPlannerTests.swift`, `WordbookMigratorTests.swift`, `WordbookExportTests.swift`, `ReviewSchedulerTests.swift`
- Current risks: default off; CSV import after enabling; large in-memory index (50k rows); `perform` unsupported — review driven from detail UI
- Sources: `Sources/LumaModules/Wordbook/WordbookModule.swift`, `Wordbook/WordbookModuleBundle.swift`

### Snippets vs Quicklinks vs Commands (doc separation)

Per `docs/MODULES.md:53-61`: Snippets expand text templates (exact trigger on Return); Quicklinks open URLs (exact first-token triggers); Commands run local scripts + built-ins. Code facts confirm separate stores/actions: `SnippetsStore`/`SnippetsAction`, `QuicklinksStore`/`QuicklinksAction`, `CommandsStore`/`CommandsAction`.

### Secrets

- Identifier: `luma.secrets` (`ModuleIdentifiers.swift:10`)
- Manifest/default: `defaultEnabled: false`, priority 1, `queryTimeout: 30ms`, `[queryable, providesActions]` (`SecretsModule.swift:5-12`); warmupTier `.hotPath`; `defaultOffNote` present (`SecretsModuleBundle.swift:6-8`)
- Triggers: `sec` (primary), `secret`/`secrets` (aliases); `bareBehavior: .openDetail` (`SecretsModuleBundle.swift:26-47`)
- Data source: Keychain (`KeychainSecretsStore`) + `secrets-metadata.json` (`SecretsVault`); values never in `handle` (`SecretsModule.swift:35-69`)
- Permissions: Keychain
- Cache/warmup: warmup configures auto-clear/relock + optionally unlocks based on `secretsRequireUnlockOnLaunch`; relock callback notifies `launcherUI` (`SecretsModule.swift:21-33`)
- Detail view: `SecretsDetailView` (`ModuleDetailRegistry.swift:104-106`)
- Core actions: unlock vault, copy secret (secure pasteboard `writeSecure` with `clearAfterSeconds`), copy account, save/update/delete (`SecretsModule.swift:71-122`)
- Hot path: Partial — targeted query; only labels, never values
- Failure/diagnostic behavior: locked row when vault locked (`SecretsModule.swift:50-52,126-142`); `SecretsVaultError.locked` → locked row; other errors → empty
- Tests: `SecretsVaultTests.swift`
- Current risks: default off; excluded from `QuerySnapshotCache` (`QuerySnapshotCache.swift:4-7`); metadata JSON quarantined via `JSONConfigPersistence`; auto-clear/relock timers from config (`Configuration.swift:98-123`)
- Sources: `Sources/LumaModules/Secrets/SecretsModule.swift`, `Secrets/SecretsModuleBundle.swift`

### Media / Records

- Identifier: `luma.media` (displayName "Records") (`ModuleIdentifiers.swift:15`)
- Manifest/default: `defaultEnabled: false`, priority 3, `queryTimeout: 30ms`, `[queryable, providesActions]` (`MediaModule.swift:5-12`); warmupTier `.onDemand` (`MediaModuleBundle.swift:5`); `defaultOffNote` present (`MediaModuleBundle.swift:6-8`)
- Triggers: `rec` (primary), `record`/`log`/`m`/`media` (aliases); `bareBehavior: .openDetail` (`MediaModuleBundle.swift:14-37`)
- Data source: `MediaStore` (`JSONFileStore`); in-memory `cachedItems` (`MediaModule.swift:14-15,166-168`)
- Permissions: None
- Cache/warmup: warmup loads `cachedItems`; `refreshCache` after mutations (`MediaModule.swift:21-23,166-168`)
- Detail view: `MediaDetailView` (`ModuleDetailRegistry.swift:107-109`)
- Core actions: capture (DSL via `MediaParser`), search, edit (draft), delete, copy summary, CSV export to `~/Downloads/luma-records-*.csv` (`MediaModule.swift:25-68,113-139`)
- Hot path: No — targeted + detail only
- Failure/diagnostic behavior: empty → manage-log row; `ModuleError.dataUnavailable` on missing item (`MediaModule.swift:84-87`)
- Tests: `MediaModuleTests.swift`, `MediaParserTests.swift`, `MediaStoreTests.swift`, `MediaIndexTests.swift`
- Current risks: default off; CSV export writes to `~/Downloads`
- Sources: `Sources/LumaModules/Media/MediaModule.swift`, `Media/MediaModuleBundle.swift`

### Commands

- Identifier: `luma.commands` (`ModuleIdentifiers.swift:7`)
- Manifest/default: `defaultEnabled: false`, priority 4, `queryTimeout: 20ms`, `[queryable, providesActions]` (`CommandsModule.swift:6-13`); warmupTier `.onDemand` (`CommandsModuleBundle.swift:5`); `defaultOffNote` present (`CommandsModuleBundle.swift:6-8`)
- Triggers: `settings`/`prefs`, `open-settings`, `reload-modules`, `exit` (built-ins); `cmd` prefix; `doctor` (`CommandsModuleBundle.swift:13-74`; `CommandsModule.swift:118-137,304-307`)
- Data source: `commands.json` (`CommandsStore`) user scripts; built-in commands; `ConfigCorruptionRegistry`; `CrashLogBuffer`; `LumaDiagnostics` (`CommandsModule.swift:17-26,188-247`)
- Permissions: None (scripts run via `ScriptRunnerService`; executable must live under `~/.luma/commands` per `docs/PERMISSIONS.md:9`)
- Cache/warmup: warmup reloads store + binds scriptRunner/currentProject/selection/reminders/menuBarTree/config (`CommandsModule.swift:38-47`)
- Detail view: None registered
- Core actions: `settings`/`open-settings`, `reload-modules`, `exit`/`quit` (host quit), `export-diagnostics`, `doctor`, run user script (with `ScriptRunnerSecurityPolicy` validation), reveal config (`CommandsModule.swift:56-105,283-296`)
- Hot path: No — targeted only; not in global search contributing set
- Failure/diagnostic behavior: `doctor` returns `LumaDiagnostics.doctorRows` (informational); script run records `CrashLogRecording` on rejection/non-zero exit (`CommandsModule.swift:85-92,188-247`)
- Tests: `CommandModulePayloadTests.swift`, `CommandsModulePerformTests.swift`, `CommandsModuleDoctorTests.swift`
- Current risks: default off means bare `exit`/`quit`/`doctor` do not respond until enabled (D-014); `doctor` is the global doctor surface; `export-diagnostics` writes `~/Library/Logs/Luma/diagnostics.json` only on explicit invocation (`ARCHITECTURE_MAP.md:871-884`)
- Sources: `Sources/LumaModules/Commands/CommandsModule.swift`, `Commands/CommandsModuleBundle.swift`

### Window Layouts

- Identifier: `luma.window-layouts` (`ModuleIdentifiers.swift:11`)
- Manifest/default: `defaultEnabled: false`, priority 3, `queryTimeout: 40ms`, `[queryable, providesActions]` (`WindowLayoutsModule.swift:5-12`); warmupTier `.hotPath`; `defaultOffNote` present (`WindowLayoutsModuleBundle.swift:6-8`)
- Triggers: `win` (primary), `wl`/`layout` (aliases) (`WindowLayoutsModuleBundle.swift:13-34`)
- Data source: `WindowLayoutCatalog` presets; focused window geometry via AX (`WindowLayoutsModule.swift:34-36`)
- Permissions: Accessibility
- Cache/warmup: warmup caches `isTrusted()` (TTL 5s); `requestAccess` invalidates cache (`WindowLayoutsModule.swift:20-23,113-128`)
- Detail view: None registered
- Core actions: apply layout preset (left/right/max/center halves/thirds), request AX, open System Settings (`WindowLayoutsModule.swift:38-52,54-93`)
- Hot path: No — targeted only
- Failure/diagnostic behavior: permission row (`PermissionResultBuilder.row`) when AX denied (`WindowLayoutsModule.swift:71-93`)
- Tests: `WindowLayoutEngineTests.swift`, `WindowLayoutsModuleTests.swift`
- Current risks: default off (D-020: no hot-path registration until warm-cache ships); AX-dependent; in `accessibilityDependentModuleIDs` (`BuiltInModules.swift:52`)
- Sources: `Sources/LumaModules/WindowLayouts/WindowLayoutsModule.swift`, `WindowLayouts/WindowLayoutsModuleBundle.swift`

### Menu Items

- Identifier: `luma.menu-items` (`ModuleIdentifiers.swift:18`)
- Manifest/default: `defaultEnabled: false`, priority 3, `queryTimeout: 800ms`, `[queryable, providesActions, backgroundUpdater]` (`MenuItemsModule.swift:10-17`); warmupTier `.onDemand` (`MenuItemsModuleBundle.swift:5`); `defaultOffNote` present (`MenuItemsModuleBundle.swift:6-8`)
- Triggers: `mb` (primary), `menu` (alias) (`MenuItemsModuleBundle.swift:13-35`)
- Data source: cached AX menu tree via `MenuBarTreeService`; `menu-items.json` config (disabled bundle IDs) (`MenuItemsModule.swift:19-32,143-159`)
- Permissions: Accessibility
- Cache/warmup: warmup loads config + `service.start(disabledBundleIDs:)`; `teardown` → `service.stop()`; AX traversal in bounded cache refreshes, not per keystroke (`MenuItemsModule.swift:34-41,50-54`)
- Detail view: None registered
- Core actions: search cached menu items, press menu item (AX via `MenuItemPresser`) (`MenuItemsModule.swift:93-129`)
- Hot path: Partial — targeted query; cache refresh scheduled when stale
- Failure/diagnostic behavior: `.degraded`/`.permissionRequired` diagnostic on empty depending on AX trust (`MenuItemsModule.swift:63-91`); `moduleHandleCold` incremented when records empty
- Tests: `MenuItemsModuleTeardownTests.swift`, `MenuItemsTests.swift`
- Current risks: default off; AX-dependent; stale cache; `menu-items.json` self-created on first load (`MenuItemsModule.swift:148-156`)
- Sources: `Sources/LumaModules/MenuItems/MenuItemsModule.swift`, `MenuItems/MenuItemsModuleBundle.swift`

### Kill Process

- Identifier: `luma.kill-process` (`ModuleIdentifiers.swift:19`)
- Manifest/default: `defaultEnabled: false`, priority 2, `queryTimeout: 150ms`, `[queryable, providesActions]` (`KillProcessModule.swift:6-13`); warmupTier `.hotPath`; `defaultOffNote` present (`KillProcessModuleBundle.swift:6-8`)
- Triggers: `kill` (primary), `quit`/`k` (aliases) (`KillProcessModuleBundle.swift:13-36`)
- Data source: running GUI apps via `RunningProcessService`; cached `[RunningProcessRecord]` TTL 3s (`KillProcessModule.swift:15-19,106-110`)
- Permissions: None
- Cache/warmup: warmup schedules refresh; `scheduleRefreshIfStale`; `teardown` not overridden (default no-op) (`KillProcessModule.swift:27-29,93-104`)
- Detail view: None registered
- Core actions: normal quit, force kill (confirm `requireSecondModifier`), relaunch, `kill refresh` (`KillProcessModule.swift:68-83,126-159`)
- Hot path: No — targeted only
- Failure/diagnostic behavior: degraded "Refreshing process list…" informational row when cold cache refresh in flight (`KillProcessModule.swift:112-124`); guarded bundle IDs require confirmation
- Tests: `KillProcessTests.swift`, `KillProcessIntegrationTests.swift`
- Current risks: default off; bare `quit`/`kill`/`k` resolve here (D-014); guarded system apps require confirmation; excludes daemons
- Sources: `Sources/LumaModules/KillProcess/KillProcessModule.swift`, `KillProcess/KillProcessModuleBundle.swift`

### Browser Tabs

- Identifier: `luma.browser-tabs` (`ModuleIdentifiers.swift:20`)
- Manifest/default: `defaultEnabled: false`, priority 3, `queryTimeout: 900ms`, `[queryable, providesActions, backgroundUpdater]` (`BrowserTabsModule.swift:6-13`); warmupTier `.onDemand` (`BrowserTabsModuleBundle.swift:5`); `defaultOffNote` present (`BrowserTabsModuleBundle.swift:6-8`)
- Triggers: `tab` (primary), `tabs` (alias) (`BrowserTabsModuleBundle.swift:13-35`)
- Data source: cached Safari/Chrome/Brave/Edge/Arc tabs via `BrowserTabsService` + `AppleScriptRunner` (`BrowserTabsModule.swift:15-19,32`)
- Permissions: Automation (AppleScript) per browser
- Cache/warmup: warmup calls `service.searchableTabs()`; background refresh; `handle` uses `service.cachedTabs()` only (`BrowserTabsModule.swift:21-23,32`)
- Detail view: None registered
- Core actions: activate tab, copy URL, save-as-Quicklink draft (`BrowserTabsModule.swift:68-112`)
- Hot path: No — targeted only; AppleScript must not run on hot path (`docs/MODULES.md:202`; test `browserTabsHandleUsesCacheOnlyPath`)
- Failure/diagnostic behavior: `.degraded` diagnostic on no tabs / no match; surfaces `service.lastDiagnostic()` (`BrowserTabsModule.swift:38-65`)
- Tests: `BrowserTabsTests.swift`, `ModuleHandleContractTests.swift` (`browserTabsHandleUsesCacheOnlyPath`)
- Current risks: default off (Automation prompts sensitive); first cold query may return no rows while background refresh runs; 900ms queryTimeout is the largest among active modules
- Sources: `Sources/LumaModules/BrowserTabs/BrowserTabsModule.swift`, `BrowserTabs/BrowserTabsModuleBundle.swift`

### Windows

- Identifier: `luma.windows` (`ModuleIdentifiers.swift:5`)
- Manifest/default: manifest `defaultEnabled: true`, priority 3, `queryTimeout: 60ms`, `[queryable, providesActions]` (`WindowsModule.swift:6-13`) — **but** not in `ModuleRegistry.allBundles` (`ModuleRegistry.swift:5-22`); only `BuiltInModules.makeDeferred()` returns `[WindowsModule()]` (`BuiltInModules.swift:48-50`). Effectively deferred, not registered for warmup/enablement.
- Triggers: none registered (no bundle in `ModuleRegistry`)
- Data source: `CGWindowListCopyWindowInfo` on-screen windows (`WindowsModule.swift:18,52-68`)
- Permissions: Screen recording API (per `docs/PERMISSIONS.md:16`)
- Cache/warmup: None — not registered
- Detail view: None registered
- Core actions: focus window (`ActionKind.focusWindow`) (`WindowsModule.swift:26-42`)
- Hot path: No — not registered
- Failure/diagnostic behavior: `handle` returns empty if no matches; no diagnostic (`WindowsModule.swift:17-24`)
- Tests: no dedicated `WindowsModule` test file found in `Tests/LumaModulesTests/**`
- Current risks: `handle` calls `CGWindowListCopyWindowInfo` directly — violates memory-only hot-path intent; deferred per `docs/MODULES.md:51` until warm-cache + tests land; manifest `defaultEnabled: true` is misleading because the module is not registered
- Sources: `Sources/LumaModules/Windows/WindowsModule.swift`, `Sources/LumaModules/BuiltInModules.swift:48-50`, `Sources/LumaModules/ModuleRegistry.swift:5-22`

### Workbench / Current Project / Capture

- Identifier: none — Workbench is **not** a `LumaModule` and has no `ModuleIdentifier`. It is a capability surfaced through the Projects module (`luma.projects`) and cross-module capture actions. (No `Workbench` entry in `ModuleIdentifiers.swift`; no `WorkbenchModuleBundle` in `ModuleRegistry.allBundles`.)
- Manifest/default: N/A
- Triggers: `proj` detail (Current Project view is the default `proj` detail, `ModuleDetailRegistry.swift:121-126`); cross-module capture actions (e.g. Clipboard → "append to note", "create snippet"; Browser Tabs → "save as Quicklink")
- Data source: `CurrentProjectService` (`Sources/LumaServices/Accessibility/CurrentProjectService.swift`, bootstrapped in `AppCoordinator.init`), `WorkbenchActivity` (`workbench-activity.json`), panel signals cache (`PanelSignalsCache`)
- Permissions: Clipboard / selection / AX context (depends on capture source)
- Cache/warmup: capture engine in `Sources/LumaModules/Workbench/WorkbenchCaptureEngine.swift` + `WorkbenchCaptureDraftBuilder.swift`; `LumaApp/Composition` orchestrators: `DefaultWorkbenchCaptureService`, `WorkbenchCaptureRunner`, `WorkbenchCommandExecutor`, `WorkbenchContextBuilder`
- Detail view: `CurrentProjectDetailView` (default `proj` detail) / `ProjectsDetailView` (manage) (`ModuleDetailRegistry.swift:116-127`)
- Core actions: convert selection/clipboard/context → note/todo/snippet/quicklink/project link; workspace row actions (`CurrentProjectWorkspaceRowAction`); project identity matching (`ProjectsModuleMatcher`)
- Hot path: No — `proj` preview must not stall on selection fetch unless attach/capture is involved (`docs/QA.md:139`)
- Failure/diagnostic behavior: `WorkbenchDiagnosticSummaryTests` exist; diagnostics summary path present
- Tests: `WorkbenchCaptureTests.swift`, `CrossModuleFlowTests.swift`, `CurrentProjectFixesTests.swift`
- Current risks: not a standalone module (no enable/disable toggle of its own); tied to Projects default-off state; `workbench-activity.json` quarantined via `JSONConfigPersistence` (`ARCHITECTURE_MAP.md:843`)
- Sources: `Sources/LumaModules/Workbench/*`, `Sources/LumaApp/Composition/DefaultWorkbenchCaptureService.swift`, `WorkbenchCaptureRunner.swift`, `WorkbenchCommandExecutor.swift`, `WorkbenchContextBuilder.swift`, `Sources/LumaApp/Launcher/CurrentProjectDetailView.swift`, `Sources/LumaServices/Accessibility/CurrentProjectService.swift`

## Code / Docs Mismatches

Recorded as observations only, no ruling.

- **`docs/PERMISSIONS.md` default column vs manifest `defaultEnabled`** — `docs/PERMISSIONS.md:5-24` marks Menu Items **on**, Wordbook **on**, Window Layouts **on**, Media **on**, Projects **on**, Kill Process **on**, Secrets **on**, Workbench **on**. Code fact: those modules' manifests all have `defaultEnabled: false` (`MenuItemsModule.swift:14`, `WordbookModule.swift:9`, `WindowLayoutsModule.swift:9`, `MediaModule.swift:9`, `ProjectsModule.swift:9`, `KillProcessModule.swift:10`, `SecretsModule.swift:9`), and `ModuleWarmupDefaults.expertDefaultOffModuleIDs` lists them as default-off (`WarmupTier.swift:47-57`). `docs/MODULES.md:20-22` and `docs/DECISIONS.md` D-012 agree with the code (default-off). The `docs/PERMISSIONS.md` "Default" column appears stale relative to D-012.
- **Windows manifest `defaultEnabled: true` vs registration** — `WindowsModule.swift:10` sets `defaultEnabled: true`, but the module is **not** in `ModuleRegistry.allBundles` (`ModuleRegistry.swift:5-22`) and only `BuiltInModules.makeDeferred()` returns it (`BuiltInModules.swift:48-50`). `docs/MODULES.md:51` says Windows is deferred and must not ship on the hot path. So the manifest flag is inconsistent with the deferred registration.
- **`docs/PERMISSIONS.md` "Menu Items" vs `docs/MODULES.md` "Menu Bar Search"** — same module `luma.menu-items`; `docs/PERMISSIONS.md:10` calls it "Menu Items" and marks it on; `docs/MODULES.md:22` calls it "Menu Bar Search" and marks it default-off. Code: `defaultEnabled: false`, displayName "Menu Bar Search" (`MenuItemsModule.swift:11-14`).
- **Diagnostics payload fields** — `docs/ENGINEERING.md:180` says `DiagnosticsExport` writes `platform`, `modules`, `permissions`, `recentErrors`, `corruptConfigFiles`. Code fact: the payload struct has all these fields, but the production call site `AppHostService.exportDiagnostics` only passes `latencyP95` and `breadcrumbs`; `platform`/`modules`/`permissions`/`recentErrors` default to `nil`/`[]` (`ARCHITECTURE_MAP.md:882-883`). Recorded as a doc/code mismatch; not ruled on.
- **`crash-log.txt` location** — `CURRENT_STATE.md` checked `~/Library/Logs/Luma/crash-log.txt` (missing). Code writes to `~/Library/Application Support/Luma/crash-log.txt` (`CrashLogBuffer.swift:26-30`). `docs/PERMISSIONS.md:29` says "Export path: `~/Library/Logs/Luma/diagnostics.json`" (correct for diagnostics) but does not document the `crash-log.txt` Application Support location. (`ARCHITECTURE_MAP.md:864-869`)
- **`LauncherContentMode` definition location** — `docs/ENGINEERING.md:85` says `LauncherContentMode` is "in `LauncherContentCoordinator`". Code fact: the enum is defined in `Sources/LumaCore/Home/LauncherKeyRouter.swift:13-31`; `LauncherContentCoordinator` holds the live instance. (`ARCHITECTURE_MAP.md:504-505`)
- **16 ms coalescer location** — `docs/ENGINEERING.md:170` says "UI snapshot apply uses the same 16 ms coalescer in `LauncherRootController`." Code fact: the 16 ms coalescing lives in `LauncherSnapshotApplyCoalescer.swift`, held by `LauncherRootController` indirectly via `snapshotPipeline` (`ARCHITECTURE_MAP.md:491-492`).
- **Bare `doctor` without `cmd` prefix** — `docs/MODULES.md:86` says bare global `doctor` does **not** run doctor checks. Code: `matchBuiltInOrDoctor` matches `"doctor"` only inside `handlePayload` (i.e. `cmd doctor` or commands-module targeted) (`CommandsModule.swift:150-151,304-307`); global `handleGlobal` uses `matchBuiltIn` which does **not** include `doctor` (`CommandsModule.swift:156-167,298-302`). Consistent, but the bare `doctor` routing was not independently re-verified end-to-end against `CommandRouter` in this phase.

## Links Back To Phase 0 / Phase 1

- **Luma app 未运行** — not a single-module issue; it is the前置 startup/running chain (`LumaApp.swift` → `AppCoordinator` → `ModuleBootstrapper.registerAndWarmup`) plus `scripts/build_app.sh` restart behavior. The module matrix records what each module does once the host is up; it does not attribute the missing process to any module. (`CURRENT_STATE.md:103-119`, `ARCHITECTURE_MAP.md:1007`)
- **hotkey p95 ≈ 8.3s** — recorded under MVP Main Path Candidates (Launcher 唤起/隐藏 is the前置 capability for all modules). Not attributed to a single module; the latency chain is `HomeLatencyTracker.markHotkey` → `LatencyTelemetry.exportReport` (`ARCHITECTURE_MAP.md:374-376,1008`). Every module's "first paint after show" depends on this.
- **`diagnostics.json` 缺失** — maps to Commands / `cmd export-diagnostics` → `AppHostService.exportDiagnostics` → `DiagnosticsExport.exportToLogsDirectory`. The file is written only on explicit invocation, so its absence is consistent with the export not having been run. (`CURRENT_STATE.md:83`, `ARCHITECTURE_MAP.md:871-884,1009`)
- **`crash-log.txt` 路径差异** — `CrashLogBuffer` writes to `~/Library/Application Support/Luma/crash-log.txt`, not `~/Library/Logs/Luma/`. Phase 0 checked the wrong directory. A file was found at the Application Support location during Phase 1. (`CURRENT_STATE.md:84`, `ARCHITECTURE_MAP.md:864-869,1010`)
- **`launcherFlowHarnessReplaysQuery` 失败** — exercises a test-only harness around `LauncherViewModel`/`QueryDispatcher`/`ModuleHost` with `BuiltInModules.makeAll()` + `warmupAll()`. Candidate factors involve Apps (async disk warmup), the empty `CommandRegistry` in the harness, and the missing `configureGlobalSearchModuleIDs` call. Not attributed to a single module. (`CURRENT_STATE.md:71`, `ARCHITECTURE_MAP.md:507-531,1012`)
- **`.ips` 崩溃 (SIGSEGV/SIGABRT)** — not attributed to any single module in this phase. `docs/swift6-appkit-boundaries.md` and `scripts/scan_appkit_executor_risk.sh` target the AppKit executor-boundary class of crash; the matrix records that `scan_appkit_executor_risk.sh` does **not** enforce `handle` memory-only constraints. (`CURRENT_STATE.md:86-88`, `ARCHITECTURE_MAP.md:1011`)

## Known Unknowns

- Whether `docs/PERMISSIONS.md`'s "Default" column is intentionally stale or should be re-synced with D-012 — recorded as mismatch, not ruled on.
- Whether any module not in `ModuleDetailRegistry.makeDefault()` (Apps, Commands, Menu Items, Kill Process, Browser Tabs, Window Layouts, Windows) has a detail view wired through a different path — not confirmed from the files read.
- Full enumeration of every call site that constructs `PermissionResultBuilder.row` across all modules — not confirmed (only Todo and Window Layouts confirmed in this phase; `ARCHITECTURE_MAP.md:629` flags this too).
- Whether `AVFoundation`/`UserNotifications` (linked in `Package.swift:37-38`) are surfaced by any module in this matrix — 未确认; `Sources/LumaServices/Speech/*` and `NotificationService.swift` exist but no module in the matrix was confirmed to use them from the files read.
- Whether `ConfigCorruptionRegistry`'s in-process list survives across app restarts or is purely runtime — 未确认 (`ARCHITECTURE_MAP.md:998`).
- Whether any code path other than `HomeLatencyTracker.markHomeRendered` under `LUMA_QA=1` writes `~/Library/Logs/Luma/latency-report.json` — 未确认 (`ARCHITECTURE_MAP.md:908,996`).
- Whether bare global `doctor` routing is fully equivalent to `cmd doctor` end-to-end — not independently re-verified against `CommandRouter` in this phase.
- Whether Windows' manifest `defaultEnabled: true` is intentional or a leftover — 未确认; only the deferred-registration fact is recorded.
- Internal behavior of `scripts/release/common.sh` (`luma_swift_build_release` / `luma_assemble_app`) — not read this phase (`ARCHITECTURE_MAP.md:991`).

## Non-Goals

This phase does **not**:

- Write any refactor plan or architectural change.
- Modify any source code, test, or existing doc.
- Fix bugs.
- Adjust any module's default on/off state.
- Delete or merge modules.
- Judge final product priority (the MVP Role column is a product classification with cited basis, not a ruling).
- Run large-scale tests. (No tests were run for this phase; facts come from reading code/docs and from Phase 0/Phase 1 observations.)
