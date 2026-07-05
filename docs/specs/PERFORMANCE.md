# Performance Requirements

## Latency Targets

| Metric | p50 | p95 | Hard ceiling |
| --- | ---: | ---: | ---: |
| Hotkey press -> interactive panel | 25 ms | 50 ms | 80 ms |
| Hotkey press -> home list rendered (Route C) | 30 ms | 50 ms | 80 ms |
| Keystroke -> first ranked snapshot painted | 12 ms | 30 ms | 60 ms |
| Module `handle` call | 5-30 ms | <= timeout | 80 ms |
| `NotesModule.handle` | ≤ 10 ms | ≤ 30 ms | 40 ms (manifest queryTimeout) |
| `SnippetsModule.handle` | ≤ 10 ms | ≤ 20 ms | 30 ms (manifest queryTimeout) |
| `SecretsModule.handle` | ≤ 10 ms | ≤ 30 ms | 40 ms (manifest queryTimeout) |
| `MediaModule.handle` | ≤ 10 ms | ≤ 20 ms | 30 ms (manifest queryTimeout) |
| Media JSON warmup load | — | ≤ 80 ms | — (≤ 5000 items) |
| Panel hide after action | 10 ms | 20 ms | 40 ms |
| `WordbookDetailView.activate` | ≤ 50 ms | ≤ 100 ms warm / ≤ 200 ms cold | — |
| `AppIndex.search` (warm, ~1k apps) | — | ≤ 5 ms | — |
| `WordbookSessionPlanner.nextCard` (warm) | — | ≤ 30 ms | — |

Launcher convergence strategy adds a stricter working rule: warm keystroke p95 above 30 ms is a regression and should block the change.

## Hot Path Rules

- Panel is instantiated at app launch.
- Result rows use stable dimensions.
- No disk or network I/O in per-keystroke module queries.
- No animated table diffs.
- App icons are cached at display size.
- Query tasks are cancelled on every new keystroke.
- Panel hides before action completion.
- Notes module reads happen only in `warmup` and on FSEvents callbacks. `handle` never touches disk.
- `n doctor` is the on-demand health surface for frontmatter, duplicate names, and broken wiki links; it may read note bodies but is not on the keystroke hot path.
- Notes warmup duration is reported in `n doctor` stats and tracked in `NotesModule.lastWarmupMilliseconds()`.
- `SelectionSnapshotService.readSelectedText` runs on a background Task; only `frontmostApplication` PID capture happens on the MainActor. AX IPC calls do not block the main thread.
- `WorkbenchContextBuilder` loads activity and link stores in parallel; command/detail query in memory only — never scan module directories.
- `ClipboardModule.handle` participates in global search with an in-memory store search capped at 3 results; it stays within the module's 30 ms query timeout.
- `WorkbenchActivityStore` reads/writes `workbench-activity.json` (v2 envelope, ≤ 50 entries) and `WorkbenchLinkStore` reads/writes `workbench-links.json` (≤ 100 links).
- `CurrentProjectDetailView.activate` shows loading model synchronously, then loads activity + link snapshots; module warmup happens only on capture execute or open detail.
- Workbench command preview (`WorkbenchCommandResults`) must not call capture or write activity; execution happens on Return via `LauncherRootController` only (`.workbench` handler never routes through `ActionExecutor`).

## Warmup

`AppCoordinator` keeps startup warmup bounded by user pins and module enablement:

1. Register all built-in bundles through `ModuleRegistry`.
2. Apply the enabled-module set.
3. Warm `pinned ∩ enabled` via `ModuleHost.warmupIfNeeded(ids:reason:)`.
4. Call `setModulesReady(true)`.
5. Only when `warmupPolicy == eagerAllEnabled`, warm the remaining enabled modules in the background.

Modules pinned to the hot path should complete warmup well within 300 ms under normal conditions. On-demand modules, especially filesystem-heavy modules such as Notes, Projects, and Menu Bar Search, must keep `handle` memory-only and perform any disk or process work in warmup, targeted commands, or detail paths.

Global search dispatch only fans out to `ModuleRegistry.globalSearchModuleIDs` (hot-path tier). Targeted commands and detail opens still call `warmupIfNeeded` for on-demand modules.

**Stale-return pattern:** `RunningApplicationsCache`, `KillProcessModule`, `ProcessMemorySampler`, and Browser Tabs use stale-while-revalidate — `handle` returns cached data immediately and schedules background refresh. TTL expiry must not block the query path on MainActor or process spawn.

**Targeted warming snapshot:** `QueryDispatcher.dispatchTargeted` emits a degraded informational row (`module.warming`) when the module is cold, then awaits warmup and delivers results. First-snapshot latency is measured separately from warmup completion.

After the panel hides, `AppCoordinator` schedules idle teardown:

1. Wait **30 seconds** after hide (cancelled if the panel reopens).
2. Call `teardownIdleModules(olderThan: 300 seconds)` — non-pinned, non-reserved warm modules only.
3. Pinned modules and the module open in detail (`reservedModuleIDs`) are never torn down by the idle pass.
4. Under memory pressure, a **60-second** idle threshold is used instead.

## Memory Budget

| Category | Lifecycle | Teardown |
| --- | --- | --- |
| hot (pinned ∩ enabled) | May stay warm after startup | Never idle-teardown |
| on-demand | Warm on targeted query, detail open, or capture-detail | hide 30s → idle 300s |
| reserved (detail) | Protected while detail is open | `closeDetail()` clears reserve → eligible for idle teardown |
| memory pressure | — | idle threshold 60s instead of 300s |
| disabled | No query, action, capture warmup | `applyEnabledSet` tears down immediately |

Capture and workbench commands must not warm unrelated modules. Only opening module detail triggers `warmupIfNeeded` for the target module.

## Measurement

Instrument these intervals first:

- `panel.show`.
- `home.render` (Route C empty-query first paint; tracked by `HomeLatencyTracker`).
- `query.keystroke_to_dispatch`.
- `query.dispatch`.
- `module.<id>.handle`.
- `ranker.score`.
- `result.render`.
- `action.execute`.

Phase 0 includes a debug latency HUD or a minimal log surface for p50/p95 while dogfooding.

`KeystrokeReplayPerformanceTests.appSearchColdPinnedWarmupReplayStaysUnderBudget` pins an explicit default hot-path set via `ConfigurationStore.setPinnedModuleIDs`, runs startup pinned warmup, then a **40-query discard batch** before measuring p95. This keeps CI stable without relaxing the 30 ms warm-replay budget.
