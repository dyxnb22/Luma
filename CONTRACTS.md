# Contracts

## Scope

This file is the **Phase 4** product of the Luma stabilization investigation, following Phase 0 (`CURRENT_STATE.md`), Phase 1 (`ARCHITECTURE_MAP.md`), Phase 2 (`MODULE_MATRIX.md`), and Phase 3 (`PRODUCT_FLOWS.md`).

It defines the **target architecture contracts** for Luma: the layering, hot-path, module-lifecycle, permission/failure, UI-ownership, concurrency, detail, cache/persistence/diagnostics, defaults, and testing/review constraints that all subsequent refactoring, bug-fixing, review, and PR acceptance must be judged against.

This document is **not**:

- An implementation plan. It does not say "change file X at step Y".
- A refactor sequence or PR ordering.
- A current-state fact map. `ARCHITECTURE_MAP.md`, `MODULE_MATRIX.md`, and `PRODUCT_FLOWS.md` are the fact maps; this file is the **target standard**.
- A ruling on which modules are ultimately kept, merged, or deleted.

Where the current code or documentation conflicts with a contract, the contract is written on the **target principle**, and the conflict is recorded under that contract's **Current Known Deviations** and in the consolidated **Current Known Deviations** section. A deviation is a recorded fact, not an accepted target: it must eventually be resolved to the contract or explicitly re-decided in `docs/DECISIONS.md`.

The intended use is as a **grading rubric**: a reviewer of any future change should first ask "does this change move toward or away from a contract in this file, and does it introduce or remove a Current Known Deviation?" before adding new local rules.

## Inputs

Phase artifacts read for this phase:

- `/Users/diaoyuxuan/Luma/CURRENT_STATE.md` (Phase 0)
- `/Users/diaoyuxuan/Luma/ARCHITECTURE_MAP.md` (Phase 1)
- `/Users/diaoyuxuan/Luma/MODULE_MATRIX.md` (Phase 2)
- `/Users/diaoyuxuan/Luma/PRODUCT_FLOWS.md` (Phase 3)

Engineering / project docs read for this phase:

- `/Users/diaoyuxuan/Luma/Package.swift`
- `/Users/diaoyuxuan/Luma/docs/ENGINEERING.md`
- `/Users/diaoyuxuan/Luma/docs/DECISIONS.md`
- `/Users/diaoyuxuan/Luma/docs/MODULES.md`
- `/Users/diaoyuxuan/Luma/docs/PERMISSIONS.md`
- `/Users/diaoyuxuan/Luma/docs/QA.md`
- `/Users/diaoyuxuan/Luma/docs/swift6-appkit-boundaries.md`
- Workspace rules: `launcher-panel-chrome`, `launcher-navigation`, `launcher-home-frozen`, `notes-detail-frozen`.

No source code or tests were modified in this phase. Contract wording is grounded in the cited phase facts and docs.

## Contract Language

- **MUST**: A hard requirement. A change that violates it cannot merge unless the violation is explicitly recorded as a Current Deviation with a tracked follow-up.
- **MUST NOT**: A prohibition. Same enforcement as MUST.
- **SHOULD**: Strongly recommended. Exceptions are allowed but must be justified in the PR description and, if durable, recorded in `docs/DECISIONS.md`.
- **MAY**: Explicitly permitted; no justification required.
- **Current Deviation**: A code or documentation fact that does not currently satisfy the contract. Listed as fact only; no fix is prescribed here.
- **Compliance Signal**: The concrete test, lint, script, doc check, manual QA step, or review checkpoint used to decide whether the contract is satisfied.

Contracts are written to be **decidable**: each has at least one Compliance Signal. Vague quality goals ("keep it simple", "improve quality") are not standalone contracts.

## Contract Index

**A. Layering**
- C-LAYER-001 `LumaApp` owns UI/window/interaction, not module business logic
- C-LAYER-002 `LumaCore` is protocols/models/pure logic/primitives, no AppKit or concrete services
- C-LAYER-003 `LumaModules` owns business modules, not AppKit or launcher view hierarchy
- C-LAYER-004 `LumaServices` wraps system APIs only
- C-LAYER-005 `LumaInfrastructure` owns logging/metrics/configuration, no user-facing flows
- C-LAYER-006 Cross-layer dependency direction is acyclic and matches `Package.swift`

**B. Hot Path / Query**
- C-HOT-001 `LumaModule.handle()` is memory-only
- C-HOT-002 No unbounded platform work on the keystroke hot path
- C-HOT-003 Global search fan-out stays small and owned
- C-HOT-004 Targeted queries return a bounded first frame
- C-HOT-005 Timeouts produce a diagnostic, never silent failure
- C-HOT-006 Query snapshot cache excludes sensitive payloads

**C. Module Lifecycle**
- C-MODULE-001 Every module declares a complete manifest
- C-MODULE-002 `warmup`/`handle`/`perform`/`teardown` responsibilities are distinct
- C-MODULE-003 Heavy work in `warmup`, memory reads in `handle`, side effects in `perform`, cancellation in `teardown`
- C-MODULE-004 Every module Task has an owner and a cancellation point
- C-MODULE-005 Default-on modules are few and stable
- C-MODULE-006 Deferred modules do not participate until warm-cache + tests + permission behavior land

**D. Permission / Failure**
- C-FAIL-001 Permission failure is user-visible
- C-FAIL-002 Cold caches show warming/degraded/cached, never fake-empty
- C-FAIL-003 Unconfigured data sources show onboarding/configuration
- C-FAIL-004 External-platform action failure never reports success
- C-FAIL-005 Diagnostic behavior is consistent across modules
- C-FAIL-006 Corruption / read-failure / quarantine is visible to doctor/diagnostics

**E. UI State Ownership**
- C-UI-001 Panel visibility has a single owner
- C-UI-002 Query text updates through a defined path
- C-UI-003 Content mode has a single owner
- C-UI-004 Selection state has a defined bridge
- C-UI-005 Detail lifecycle uses defined entry/exit paths
- C-UI-006 No bypassing an owner to mutate shared UI state

**F. AppKit / MainActor / Async**
- C-APPKIT-001 AppKit overrides follow Swift 6 executor-boundary rules
- C-APPKIT-002 C/Carbon/AppKit callbacks use nonisolated entry + MainActor bridge
- C-APPKIT-003 `Task { @MainActor }` is a boundary bridge, not a state-fix hammer
- C-ASYNC-001 Every Task has an owner, cancellation, and generation/lifecycle guard
- C-ASYNC-002 Show/hide/detail/restore/snapshot-apply use generation guards
- C-ASYNC-003 Background refresh does not repaint hidden panel or block hotkey→visible

**G. Detail**
- C-DETAIL-001 Detail views do not drive module lifecycle in reverse
- C-DETAIL-002 Detail open goes through presenter/registry/coordinator
- C-DETAIL-003 Detail reuse/pooling keeps a generation/content-revision contract
- C-DETAIL-004 Detail exit goes through the exit planner
- C-DETAIL-005 Modules without a registered detail do not advertise open-detail

**H. Cache / Persistence / Diagnostics**
- C-CACHE-001 Every cache has owner, scope, and invalidation
- C-CACHE-002 Caches never hide permission/corruption/missing-source
- C-PERSIST-001 Every persistence format defines corruption/read-failure behavior
- C-PERSIST-002 Doctor/diagnostics can report key config/store corruption
- C-DIAG-001 Diagnostics export is reachable, stable, redacted, local-only
- C-DIAG-002 Declared diagnostics fields have real population sources
- C-DIAG-003 Crash-breadcrumb write failure is isolated but explainable
- C-DIAG-004 Diagnostics and crash-log paths are documented and consistent

**I. Defaults / Product Surface**
- C-DEFAULT-001 Default-on set is few and stable
- C-DEFAULT-002 Default-on modules are useful without high-sensitivity permissions
- C-DEFAULT-003 High-permission/expert/heavy modules are default-off or deferred
- C-DEFAULT-004 Defaults are consistent across docs, manifest, and warmup defaults
- C-DEFAULT-005 Diagnostics/recovery entry is not fully cut off by default config

**J. Testing / Review**
- C-TEST-001 Every P0 flow has an automated test or explicit manual QA
- C-TEST-002 Every module has handle-contract coverage
- C-TEST-003 Every AppKit executor-boundary rule has a lint/test defense
- C-TEST-004 Every main path maps to `PRODUCT_FLOWS.md`
- C-REVIEW-001 Review judges against CONTRACTS.md before adding local rules

---

## A. Layering Contracts

### C-LAYER-001: `LumaApp` Owns UI, Windows, And Interaction — Not Module Business Logic

**Contract**
`LumaApp` MUST own only app lifecycle, the AppKit launcher and panel, hotkey, settings, detail view implementations, and user-interaction scheduling. It MUST NOT own module business logic (search/index/store/action semantics). Module data and actions live in `LumaModules`; `LumaApp` reaches module state only through defined composition seams (e.g. `ModuleDetailRegistry`, `ModuleHost`, `ActionExecutor`, `HostClient`).

**Rationale**
- `docs/ENGINEERING.md` layer-ownership table: `LumaApp` owns "App lifecycle, AppKit launcher, detail views, hotkey, settings"; must not own "Module business logic".
- `ARCHITECTURE_MAP.md` records `AppCoordinator` as the single top-level object owning nearly every long-lived service/controller with "no formal DI container", which increases the risk of business logic leaking into the app layer.
- `PRODUCT_FLOWS.md` Cross-Cutting State Owners table names the app-layer owners (`LauncherWindowController`, `LauncherContentCoordinator`, `LauncherDetailPresenter`) that must stay UI-scoped.

**Applies To**
- `Sources/LumaApp/**`
- `AppCoordinator`, `LauncherWindowController`, `LauncherRootController`, detail views
- Composition seams: `ModuleDetailRegistry`, `AppHostService`

**Compliance Signals**
- Review checkpoint: new business logic (ranking, store mutation, index building, action semantics) added under `Sources/LumaApp/**` is a violation.
- Grep/import audit: detail views read module instances through `ModuleDetailRegistry`, not by importing module internals directly.
- `docs/ENGINEERING.md` layer table remains the cited source of truth for any PR touching layer placement.

**Current Known Deviations**
- `AppCoordinator` constructs a dedicated `ProjectsModule` instance for path matching before `ModuleHost` registers the module (`PRODUCT_FLOWS.md` Flow 1), i.e. module-adjacent logic is instantiated in the app layer; recorded as fact, not ruled on.
- Workbench capture orchestration is split between `LumaModules/Workbench` and `LumaApp/Composition` (`MODULE_MATRIX.md` Workbench section), so some capture business logic sits in the app layer.

### C-LAYER-002: `LumaCore` Is Protocols, Models, Pure Logic, And Primitives

**Contract**
`LumaCore` MUST contain only protocols, models, pure logic, state machines, ranking, query routing, and generic persistence/security/diagnostic **primitives**. It MUST NOT import AppKit, and MUST NOT depend on concrete service implementations or on `LumaModules`/`LumaApp`.

**Rationale**
- `docs/ENGINEERING.md` layer table: `LumaCore` owns "Protocols, models, query, ranking, actions, persistence helpers, design tokens"; must not own "AppKit detail implementations".
- `Package.swift`: `LumaCore` target has **no** dependencies — it is the root of the dependency graph.
- `ARCHITECTURE_MAP.md` confirms pure decision types live in `LumaCore` (`LauncherPanelVisibilitySession`, `LauncherDetailExitPlanner`, `LauncherQueryDispatchPolicy`, `QueryDispatcher`, `ModuleHost`).

**Applies To**
- `Sources/LumaCore/**`
- `LumaModule` protocol, `QueryDispatcher`, `ModuleHost`, `ActionExecutor`, `JSONConfigPersistence`, `ConfigCorruptionRegistry`, `DiagnosticsExport`

**Compliance Signals**
- Build/import audit: no `import AppKit` (except `@preconcurrency` where a strictly UI-adjacent primitive is unavoidable and documented) and no dependency edge from `LumaCore` to any other Luma target in `Package.swift`.
- Review checkpoint: concrete AppKit detail views must not be placed in `LumaCore`.

**Current Known Deviations**
- ~~`LauncherContentMode` doc location~~ — **Resolved P3.1 (2026-07-07):** type in `LauncherKeyRouter.swift`; runtime owner `LauncherContentCoordinator` (`docs/ENGINEERING.md`).
- ~~Diagnostics layer attribution stale~~ — **Resolved P3.1 (2026-07-07):** `DiagnosticsPayload`/`DiagnosticsExport`/`CrashLogRecording` in `LumaCore/Util`; `CrashLogBuffer` in `LumaInfrastructure` (`docs/ENGINEERING.md`, `docs/PERMISSIONS.md`).

### C-LAYER-003: `LumaModules` Owns Business Modules — Not AppKit Or Launcher View Hierarchy

**Contract**
`LumaModules` MUST own built-in module actors, their stores/indexes/actions, and their manifests. It MUST NOT directly touch AppKit, MUST NOT manage the launcher view hierarchy, and MUST NOT import or call `ModuleDetailRegistry` / `LumaApp`.

**Rationale**
- `docs/ENGINEERING.md` layer table: `LumaModules` owns "Built-in module actors, module stores/indexes, module actions"; must not own "Launcher view hierarchy". It also states "Modules do not import or call the registry."
- `Package.swift`: `LumaModules` depends only on `LumaCore` and `LumaServices` (no `LumaApp`).
- `MODULE_MATRIX.md` documents that detail views are registered in `LumaApp` via `ModuleDetailRegistry`, keeping module actors free of view hierarchy.

**Applies To**
- `Sources/LumaModules/**`
- All `*Module.swift` / `*ModuleBundle.swift`

**Compliance Signals**
- Import audit: no `import AppKit`, no reference to `ModuleDetailRegistry` or `LumaApp` types from `Sources/LumaModules/**`.
- `Package.swift` dependency edges unchanged (no `LumaApp` edge added to `LumaModules`).
- Review checkpoint for any new module.

**Current Known Deviations**
- `WindowsModule.handle` directly calls `CGWindowListCopyWindowInfo` (`MODULE_MATRIX.md` Windows section). This is a system-API call inside a module rather than being routed through a `LumaServices` wrapper (see also C-LAYER-004, C-HOT-001); the module is deferred/unregistered.

### C-LAYER-004: `LumaServices` Wraps System APIs Only

**Contract**
`LumaServices` MUST encapsulate system API access (Accessibility/AX, CGWindow, EventKit, Keychain, AppleScript, Pasteboard, Workspace, FSEvents, processes, Translation, notifications). It MUST NOT contain product routing or module business logic. Modules and the app MUST reach system APIs through these wrappers, not by calling the platform directly.

**Rationale**
- `docs/ENGINEERING.md` layer table: `LumaServices` owns "System API wrappers for AX, CGWindow, EventKit, Keychain, AppleScript, processes"; must not own "Product routing".
- `Package.swift`: `LumaServices` depends on `LumaCore`/`LumaInfrastructure` and links the system frameworks (`Translation`, `EventKit`, `AVFoundation`, `UserNotifications`).
- `MODULE_MATRIX.md` permission map shows the intended wrapper set (`AXService`, `RemindersService`, `AppleScriptRunner`, `KeychainSecretsStore`, `PasteboardService`, `WorkspaceService`, `FSEventsService`, `RunningProcessService`).

**Applies To**
- `Sources/LumaServices/**`
- All module/system-API touch points

**Compliance Signals**
- Grep audit: direct platform calls (`CGWindowListCopyWindowInfo`, raw `EKEventStore`, raw AppleScript, raw Keychain) outside `LumaServices` are violations.
- Review checkpoint: new system access is added as a service wrapper.

**Current Known Deviations**
- `WindowsModule.handle` calls `CGWindowListCopyWindowInfo` directly instead of going through a `LumaServices` wrapper (`MODULE_MATRIX.md`); module is deferred.
- `AVFoundation`/`UserNotifications` are linked but no module in `MODULE_MATRIX.md` was confirmed to surface them — wrapper ownership for these is unconfirmed (`MODULE_MATRIX.md` Known Unknowns).

### C-LAYER-005: `LumaInfrastructure` Owns Logging, Metrics, Configuration — No User-Facing Flows

**Contract**
`LumaInfrastructure` MUST own logging, metrics/signposting, and configuration primitives. It MUST NOT own user-facing flows or product routing.

**Rationale**
- `docs/ENGINEERING.md` layer table: `LumaInfrastructure` owns "Logging, metrics, configuration"; must not own "User-facing flows".
- `Package.swift`: `LumaInfrastructure` depends only on `LumaCore`.
- `ARCHITECTURE_MAP.md`: `Signposter.swift`/`LumaMetrics` live only in `LumaInfrastructure`; `LauncherPerfCounters`/`LauncherDurationRecorder` under `LumaInfrastructure` are re-export typealiases of the `LumaCore` implementations.

**Applies To**
- `Sources/LumaInfrastructure/**`
- `LumaLogger`, `LumaMetrics`, `Configuration`, `CrashLogBuffer`

**Compliance Signals**
- Import/dependency audit: `LumaInfrastructure` keeps only the `LumaCore` edge; no launcher/flow types added.
- Review checkpoint on any file added under `Sources/LumaInfrastructure/**`.

**Current Known Deviations**
- None recorded. Diagnostics split is intentional and documented (P3.1): `CrashLogBuffer` in `LumaInfrastructure`; `DiagnosticsPayload`/`DiagnosticsExport`/`CrashLogRecording` in `LumaCore/Util`; assembly in `LumaApp` (`RecoveryDiagnosticsCollector`, `AppHostService`).

### C-LAYER-006: Cross-Layer Dependency Direction Is Acyclic And Matches `Package.swift`

**Contract**
The dependency direction MUST remain acyclic and follow `Package.swift`: `LumaCore` depends on nothing; `LumaInfrastructure → LumaCore`; `LumaServices → LumaCore, LumaInfrastructure`; `LumaModules → LumaCore, LumaServices`; `LumaApp → all`. No lower layer MUST import a higher layer, and no dependency cycle MUST be introduced.

**Rationale**
- `Package.swift` target graph (`ARCHITECTURE_MAP.md` Package And Target Graph) defines exactly this ordering.
- Acyclic layering is what keeps `handle`-time hot paths, module isolation, and Swift 6 concurrency reasoning tractable.

**Applies To**
- `Package.swift`
- All target dependency edges

**Compliance Signals**
- `swift build` plus a review check that no new dependency edge inverts the direction (e.g. `LumaCore` gaining a dependency, `LumaModules` importing `LumaApp`).
- `Package.swift` diff review on any dependency change.

**Current Known Deviations**
- `LumaCoreTests` depends on `LumaModules` (`Package.swift`), so some core-layer tests exercise modules; this is a test-target edge, noted as fact (`ARCHITECTURE_MAP.md`). No production cycle was recorded.

---

## B. Hot Path / Query Contracts

### C-HOT-001: `LumaModule.handle()` Is Memory-Only

**Contract**
`LumaModule.handle()` MUST answer from already-warmed in-memory state. It MUST NOT perform disk scans, large JSON reads, AX traversal, AppleScript, EventKit fetches, process enumeration, network calls, or other unbounded platform work. Cold internal caches MUST return a warming/degraded/permission row and schedule a background refresh instead of doing the heavy work inline.

**Rationale**
- `docs/ENGINEERING.md` Query And Module Contract: "`handle`: answer from memory only; no disk, network, AppleScript, AX traversal, process enumeration, or large JSON parsing."
- `ARCHITECTURE_MAP.md` / `MODULE_MATRIX.md`: this constraint is documentation-only, not enforced by the type system or static analysis; `QueryContext` carries only a deadline + platform clients and does not sandbox module code.
- `MODULE_MATRIX.md`: `WindowsModule.handle` calls `CGWindowListCopyWindowInfo` directly, the one confirmed source-tree violation.
- `PRODUCT_FLOWS.md` Flows 6/7 describe global and targeted hot paths that call `handle` per keystroke.

**Applies To**
- All `LumaModule` implementations
- `QueryDispatcher.dispatch` (global) and `QueryDispatcher.dispatchTargeted` (targeted)
- All global-search contributing modules and all targeted modules

**Compliance Signals**
- `Tests/LumaModulesTests/ModuleHandleContractTests.swift` (`snippetsHandleDoesNotAwaitAccessibility`, `browserTabsHandleUsesCacheOnlyPath`) and `ModuleColdCacheTests.swift`.
- Source scan for forbidden calls inside any `handle` (disk/AX/AppleScript/process/network/large-JSON).
- Performance tests (`AppsModuleTopQueryPerformanceTests`, `KeystrokeReplayPerformanceTests`, `ColdTargetedFirstSnapshotPerformanceTests`).
- Review checklist item for every new module.

**Current Known Deviations**
- `WindowsModule.handle` calls `CGWindowListCopyWindowInfo` directly (deferred, not registered in `ModuleRegistry.allBundles`).
- No generic type-system/static enforcement of `handle` memory-only exists; only per-module proxy tests cover it (`MODULE_MATRIX.md` Hot Path And Blocking Risk).

### C-HOT-002: No Unbounded Platform Work On The Keystroke Hot Path

**Contract**
Disk scans, AX traversal, AppleScript, EventKit fetches, process enumeration, and large JSON parsing MUST NOT run on the keystroke hot path (per-keystroke query dispatch, warm-up-on-query first frame). Such work MUST be relocated to `warmup`, bounded background refresh tasks, or explicit `refresh`/`doctor` commands.

**Rationale**
- `docs/ENGINEERING.md` Performance Contract hot-path rules: "Browser Tabs must not await AppleScript on the keystroke path", "Kill Process must not do process memory sampling on MainActor", "Notes and Projects must query memory indexes, not scan disk", "Menu Bar Search must query a cache, not traverse AX per keystroke."
- `docs/ENGINEERING.md` Performance Contract budgets: keystroke → first ranked paint 30 ms p95 / 60 ms ceiling; module `handle` 80 ms ceiling.
- `PRODUCT_FLOWS.md` Flow 5/6/7 confirm the per-keystroke dispatch path and the per-module `Timeout.run` mechanism.

**Applies To**
- `QueryDispatcher.dispatch` / `dispatchTargeted` and every module's `handle`
- Browser Tabs, Kill Process, Notes, Projects, Menu Items specifically (named in docs)

**Compliance Signals**
- `swift test --filter KeystrokeReplayPerformanceTests`, `AppsModuleTopQueryPerformanceTests`, `QueryDispatcher`, `ColdTargetedFirstSnapshotPerformanceTests`.
- Per-module cache-only handle tests (`browserTabsHandleUsesCacheOnlyPath`, `snippetsHandleDoesNotAwaitAccessibility`).
- `latency-report.json` `keystrokeP95Milliseconds` under the documented budget (Phase 0 baseline ≈ 19.13 ms is within budget).

**Current Known Deviations**
- Phase 0 `hotkeyP95Milliseconds` ≈ 8.3 s (far above the 50 ms p95 / 80 ms ceiling for hotkey→interactive/home). This is a show-path latency deviation, not a per-keystroke one, but it is the strongest evidence that the hot-path budget is currently unmet somewhere in the show/first-frame chain (`CURRENT_STATE.md`, `PRODUCT_FLOWS.md` Flows 3/4).
- `WindowsModule.handle`'s direct CGWindow call would violate this on the hot path if registered (deferred).

### C-HOT-003: Global Search Fan-Out Stays Small And Owned

**Contract**
Global (unprefixed) search fan-out MUST be limited to an explicit, small set of **contributing** modules, each with a named owner and a performance budget. Adding a module to the global fan-out is a deliberate decision, not an implicit consequence of the module being hot-path or enabled.

**Rationale**
- `docs/ENGINEERING.md` Performance Contract: "Global search fan-out is limited to **contributing** modules (`apps`, `quicklinks`, `clipboard`); other hot-path modules return empty for unprefixed queries and are not scheduled."
- `MODULE_MATRIX.md` / `ARCHITECTURE_MAP.md`: production fan-out is `GlobalSearchTiers.contributingModuleIDs` = apps, quicklinks, clipboard, applied via `ModuleBootstrapper` → `host.configureGlobalSearchModuleIDs(...)`; fast tier = apps, quicklinks; deferred tier = clipboard.
- `docs/ENGINEERING.md` also requires ≥2 characters for global search unless prefixed.

**Applies To**
- `ModuleRegistry.globalSearchModuleIDs`, `GlobalSearchTiers`, `ModuleHost.enabledQueryableModules(forGlobalSearch:)`
- `QueryDispatcher.performGlobalSearch`

**Compliance Signals**
- `swift test --filter GlobalSearchDispatch`, `QueryDispatcherTier`, `QuerySnapshot`.
- Review checkpoint: any change to the contributing set updates `docs/ENGINEERING.md` and `docs/MODULES.md` "Global Search" column.
- `docs/MODULES.md` Active Modules "Global Search" column stays consistent with `GlobalSearchTiers`.

**Current Known Deviations**
- None recorded as an active violation; the fan-out is currently narrowed as documented. Snippets is fast-tier but not contributing (participates only via exact-trigger expansion) — this is the documented design, noted for clarity, not a deviation.

### C-HOT-004: Targeted Queries Return A Bounded First Frame

**Contract**
A targeted (prefixed) query MAY trigger warmup, but the first snapshot MUST be a bounded response: a warming row, a diagnostic row, or a cached result. The user MUST NOT see an indefinite blank while warmup or `handle` runs.

**Rationale**
- `docs/ENGINEERING.md`: "Targeted cold modules may emit a warming or refreshing informational row"; "Fire-and-forget cache refreshes must still show a clear cold-cache state if the user queries before data arrives."
- `PRODUCT_FLOWS.md` Flow 7: `dispatchTargeted` immediately emits a `module.warming` informational row when the module is `.cold` before warmup starts, then a follow-up snapshot when `handle` returns.
- `MODULE_MATRIX.md` Diagnostic Requirements: per-module cold rows (Apps "Memory usage cache warming", Kill Process "Refreshing process list…", Wordbook "Loading due words…", Menu Items degraded).

**Applies To**
- `QueryDispatcher.dispatchTargeted`
- Every targeted module's cold-state `handle` output

**Compliance Signals**
- `swift test --filter ColdTargetedFirstSnapshotPerformanceTests`, `Tests/LumaModulesTests/ModuleColdCacheTests.swift`.
- `docs/QA.md` manual: targeted cold module shows warming/refreshing row then content.
- Review checkpoint for new targeted modules' cold behavior.

**Current Known Deviations**
- Non-`.queryable` targeted modules return a silent empty snapshot (no diagnostic) — a different behavior from the disabled-module diagnostic row (`PRODUCT_FLOWS.md` Flow 7). Recorded as an inconsistency to resolve under C-FAIL-005.

### C-HOT-005: Timeouts Produce A Diagnostic, Never Silent Failure

**Contract**
A per-module query timeout MUST produce a diagnostic row/status, not a silent empty result. Timeout handling MUST be consistent between global and targeted dispatch.

**Rationale**
- `docs/ENGINEERING.md`: "Disabled or permission-blocked modules return diagnostic rows, not silent empty results."
- `PRODUCT_FLOWS.md` Flows 6/7 + `MODULE_MATRIX.md`: `Timeout.run` returning `nil` synthesizes `ModuleResult.empty(for:diagnostic: .timeout "Module timed out")`, records `CrashLogRecording`, increments warmup-timeout counters; `mergeItems` turns an empty-result-with-diagnostic into a top-ranked (score 1000) diagnostic row.

**Applies To**
- `QueryDispatcher` timeout path (`Timeout.run`, `mergeItems`)
- `manifest.queryTimeout` values per module

**Compliance Signals**
- `swift test --filter TimeoutTests`, `QueryDispatcher`.
- Review checkpoint: a module handle change that can silently return empty on slow paths (instead of a diagnostic) is a violation.

**Current Known Deviations**
- A module returning zero items **with no diagnostic** contributes nothing and is silently absent from the merged list (`PRODUCT_FLOWS.md` Flow 6). This is documented as "empty acceptable" for some modules but is the boundary case where "silent empty" and "acceptable empty" overlap; per-module classification lives in `MODULE_MATRIX.md` Diagnostic Requirements and is not uniformly enforced (see C-FAIL-005).

### C-HOT-006: Query Snapshot Cache Excludes Sensitive Payloads

**Contract**
The query snapshot cache MUST have an explicit exclusion policy. Modules handling secrets, snippet bodies, or otherwise sensitive payloads MUST NOT have their sensitive content cached in the query snapshot cache.

**Rationale**
- `docs/ENGINEERING.md`: "Only **secrets** and **snippets** are excluded from query cache (clipboard may appear in cached global snapshots)."
- `docs/DECISIONS.md`: `QuerySnapshotCache` is separate from `UsageResultCache`; secrets/snippets excluded.
- `MODULE_MATRIX.md`: Secrets and Snippets excluded from `QuerySnapshotCache`; Secrets values never in `handle`; clipboard has privacy filters.

**Applies To**
- `QuerySnapshotCache`
- Secrets, Snippets modules; Clipboard privacy filters

**Compliance Signals**
- `swift test --filter QuerySnapshot`, `SecretsVaultTests`.
- Review checkpoint: adding a sensitive module to the query cache without an exclusion is a violation.
- `docs/DECISIONS.md` cache-exclusion note stays consistent with the code exclusion set.

**Current Known Deviations**
- None recorded as an active violation. Clipboard is cacheable by design but relies on its own privacy filters (secret-looking values, blocked bundles, size/age) rather than cache exclusion; correctness of those filters is a Clipboard-specific concern tracked under C-CACHE-002.

---

## C. Module Lifecycle Contracts

### C-MODULE-001: Every Module Declares A Complete Manifest

**Contract**
Every registered module MUST declare a manifest and bundle covering: identifier, triggers/aliases/bare behavior, default enabled state, warmup tier, capabilities, query timeout, required permissions, and failure/diagnostic behavior. These declarations MUST be the single source of truth consumed by registration, warmup, routing, and defaults.

**Rationale**
- `MODULE_MATRIX.md` Summary Matrix: each module's manifest carries `defaultEnabled`, priority, `queryTimeout`, capabilities; each bundle carries triggers/aliases/`bareBehavior`/`warmupTier`.
- `docs/ENGINEERING.md` Query And Module Contract: "`manifest`: static metadata."
- `MODULE_MATRIX.md` documents warmup-tier and default-state cross-checks against `WarmupTier.swift`.

**Applies To**
- All `*Module.swift` manifests and `*ModuleBundle.swift`
- `ModuleRegistry`, `ModuleWarmupDefaults`

**Compliance Signals**
- `swift test --filter BuiltInModules`, module-specific tests.
- Review checkpoint: a new module without complete manifest/bundle fields is incomplete.
- `MODULE_MATRIX.md` Summary Matrix regenerable from manifests.

**Current Known Deviations**
- `WindowsModule` remains deferred/unregistered; `handle` calls `CGWindowListCopyWindowInfo` directly (`MODULE_MATRIX.md`). Manifest `defaultEnabled: false` since P2.1.

### C-MODULE-002: `warmup` / `handle` / `perform` / `teardown` Responsibilities Are Distinct

**Contract**
The four module lifecycle methods MUST keep distinct responsibilities and MUST NOT be collapsed: `warmup` prepares state, `handle` reads it, `perform` mutates the outside world, `teardown` releases resources.

**Rationale**
- `docs/ENGINEERING.md` Query And Module Contract enumerates the four with distinct budgets: warmup soft budget 1 s, handle memory-only, perform soft budget 2 s, teardown cancels background work.
- `PRODUCT_FLOWS.md` Flow 14 and `MODULE_MATRIX.md` per-module notes confirm the current split (warmup builds indexes; handle reads caches; perform runs actions; teardown cancels tasks).

**Applies To**
- All `LumaModule` implementations
- `ModuleHost` (warmup/teardown), `ActionExecutor` (perform), `QueryDispatcher` (handle)

**Compliance Signals**
- `swift test --filter ModuleHostWarmupPolicy`, `ModuleHostReentrancy`, per-module teardown tests (`MenuItemsModuleTeardownTests`).
- Review checkpoint for new modules: no side effects in `handle`, no query answering in `perform`.

**Current Known Deviations**
- `KillProcessModule` does not override `teardown` (default no-op) though it schedules refresh tasks (`MODULE_MATRIX.md`); task ownership/cancellation for it is via TTL/staleness rather than explicit teardown.
- `WordbookModule.perform` throws `unsupportedAction` (review runs in detail) — an intentional narrowing, noted as fact.

### C-MODULE-003: Heavy Work In `warmup`, Memory Reads In `handle`, Side Effects In `perform`, Cancellation In `teardown`

**Contract**
`warmup` MAY do heavy work (within its 1 s soft budget); `handle` MUST read caches only (see C-HOT-001); `perform` MUST be where side effects execute (within the 2 s soft budget via `ActionExecutor`); `teardown` MUST cancel background tasks and flush state.

**Rationale**
- `docs/ENGINEERING.md` Query And Module Contract budgets and semantics.
- `PRODUCT_FLOWS.md` Flow 9: `ActionExecutor.run` races `.custom` `module.perform` against a 2 s `Timeout.run`.
- `PRODUCT_FLOWS.md` Flow 14: `warmupIfNeeded` races `warmup` against a 1 s budget; a timed-out warmup never marks `.warm`.

**Applies To**
- All `LumaModule` implementations
- `ActionExecutor`, `ModuleHost`, `Timeout`

**Compliance Signals**
- `swift test --filter QueryDispatcher`, `ModuleHostWarmupPolicy`, `TimeoutTests`, per-module perform tests (`CommandsModulePerformTests`, `PasteOutcomeTests`).
- Performance/cold-cache tests.

**Current Known Deviations**
- Warmup timeouts are absorbed silently (module stays `.cold`) and are surfaced only later as cold-state UI rows; there is no user-visible "warmup failed" distinct from "warming" (`PRODUCT_FLOWS.md` Flow 14). Noted as fact under C-FAIL-002/C-FAIL-005.

### C-MODULE-004: Every Module Task Has An Owner And A Cancellation Point

**Contract**
Every background `Task` a module starts MUST have a clear owner and MUST be cancelled at the corresponding lifecycle point (teardown, disable, or app lifecycle). No module Task may outlive its module's teardown.

**Rationale**
- `docs/ENGINEERING.md`: "`teardown`: cancel background work and flush state"; "Idle teardown after hide should release on-demand module resources."
- `PRODUCT_FLOWS.md` Flow 14 + `MODULE_MATRIX.md`: per-module tasks enumerated (Apps refresh loops, Notes FSEvents watch, Todo due/store-change tasks, Wordbook data/due tasks, Clipboard polling) with teardown cancellation.
- `docs/PERMISSIONS.md` Settings toggle: disabling a module tears down the module actor and cancels active work.

**Applies To**
- All module background tasks
- `ModuleHost.teardownIdleModules`, `AppCoordinator` idle/memory-pressure teardown

**Compliance Signals**
- `swift test --filter ModuleHostWarmupPolicy`, `MenuItemsModuleTeardownTests`, `TodoModuleStoreChangesTests`.
- Review checkpoint: any new module task shows its cancellation site.

**Current Known Deviations**
- `KillProcessModule` has no explicit teardown for its refresh scheduling (`MODULE_MATRIX.md`); relies on TTL.
- The menu-tree context Task started in the show path (300 ms sleep) is "not re-verified whether it self-cancels on hide" (`PRODUCT_FLOWS.md` Flow 3) — owner/cancellation unconfirmed.

### C-MODULE-005: Default-On Modules Are Few And Stable

**Contract**
The set of modules that warm up and answer on a fresh install MUST be small and stable. High-permission, high-risk, or expert modules MUST default off or be deferred. (See also C-DEFAULT-001/003.)

**Rationale**
- `docs/DECISIONS.md` D-012: seven default-on MVP modules (Apps, Clipboard, Snippets, Quicklinks, Todo, Translate, Notes); six/nine expert modules default off.
- `docs/MODULES.md` MVP default-on list and default-off list.
- `MODULE_MATRIX.md` cross-checks manifest `defaultEnabled` against `ModuleWarmupDefaults`.

**Applies To**
- `ModuleWarmupDefaults.defaultEnabledModuleIDs` / `expertDefaultOffModuleIDs`
- Each module's manifest `defaultEnabled`

**Compliance Signals**
- `swift test --filter BuiltInModules`, migration tests for schema v2.
- `docs/DECISIONS.md` D-012 and `docs/MODULES.md` lists match manifests.

**Current Known Deviations**
- None recorded for the D-012 default-off set (`docs/PERMISSIONS.md` aligned P2.1).

### C-MODULE-006: Deferred Modules Do Not Participate Until Warm-Cache + Tests + Permission Behavior Land

**Contract**
A deferred module MUST NOT be registered, warmed, or allowed to participate in query dispatch until it has (a) a warm-cache/memory-only `handle`, (b) tests, and (c) defined permission/failure behavior. Its manifest MUST NOT claim a default-on state while deferred.

**Rationale**
- `docs/MODULES.md`: "Deferred source-retained module: **Windows** ... Not registered in active warmup/default enablement; `handle()` must not ship on the hot path until warm-cache + tests land."
- `MODULE_MATRIX.md` Windows section: deferred, `handle` violates memory-only, no dedicated test file. *(Phase 0 historical: manifest claimed `defaultEnabled: true` — corrected P2.1 to `false`.)*

**Applies To**
- `WindowsModule` and any future deferred module
- `ModuleRegistry.allBundles`, `BuiltInModules.makeDeferred()`

**Compliance Signals**
- Registration audit: deferred modules absent from `ModuleRegistry.allBundles`.
- Review gate: promoting a deferred module requires memory-only `handle` + tests + permission behavior.

**Current Known Deviations**
- `WindowsModule` `handle` calls `CGWindowListCopyWindowInfo` while deferred; no dedicated test (`MODULE_MATRIX.md`). Manifest metadata `defaultEnabled: false` since P2.1.

---

## D. Permission / Failure Contracts

### C-FAIL-001: Permission Failure Is User-Visible

**Contract**
When a query/detail/action touches a permission-gated surface (Accessibility, EventKit, Automation, Keychain) that is denied/not-determined/locked, the failure MUST surface a user-visible diagnostic row, status, or banner. It MUST NOT be a silent empty result.

**Rationale**
- `docs/ENGINEERING.md`: "Disabled or permission-blocked modules return diagnostic rows, not silent empty results"; "Accessibility permission is lazy: show the banner only on AX-dependent surfaces."
- `docs/QA.md` Permissions checks: AX banner on AX-dependent surfaces; Browser Tabs Automation denial actionable; Todo/EventKit denial actionable.
- `MODULE_MATRIX.md` / `PRODUCT_FLOWS.md` Flow 13: Todo/Window Layouts permission rows, Menu Items/Browser Tabs degraded diagnostics, Snippets/Clipboard `permissionRequired`, Secrets locked row, AX banner via `PermissionBannerController`.

**Applies To**
- `PermissionBannerController`, `AccessibilityGuidancePolicy`, `PermissionResultBuilder.row`, `ModuleDiagnosticResults`
- Todo, Window Layouts, Menu Items, Browser Tabs, Snippets, Clipboard, Secrets

**Compliance Signals**
- `swift test --filter PermissionBanner` (`PermissionBannerContextTests`), per-module permission tests (Todo EventKit, Browser Tabs diagnostics).
- `docs/QA.md` Permissions manual checks.

**Current Known Deviations**
- Full enumeration of `PermissionResultBuilder.row` call sites is unconfirmed beyond Todo and Window Layouts (`ARCHITECTURE_MAP.md`, `MODULE_MATRIX.md` Known Unknowns) — coverage of the contract across all permissioned modules is not fully verified.

### C-FAIL-002: Cold Caches Show Warming/Degraded/Cached, Never Fake-Empty

**Contract**
A cold cache MUST produce a warming/degraded/cached result that is visibly distinct from "there are genuinely no results." The user MUST be able to tell "still loading" from "nothing found."

**Rationale**
- `docs/ENGINEERING.md`: "Fire-and-forget cache refreshes must still show a clear cold-cache state if the user queries before data arrives"; Launcher Contract distinguishes `module.warming` from `home.openApps.empty`.
- `MODULE_MATRIX.md` Diagnostic Requirements: Apps `app top` cold, Kill Process cold, Wordbook cold, Menu Items empty-cache degraded rows.

**Applies To**
- All warm-cache modules
- `LauncherHomeAggregator` Open Apps empty-vs-warming distinction

**Compliance Signals**
- `swift test --filter ColdTargetedFirstSnapshot`, `Tests/LumaModulesTests/ModuleColdCacheTests.swift`.
- `docs/QA.md` performance/first-frame manual checks.

**Current Known Deviations**
- Warmup timeout marks the module `.cold` silently; the distinction between "warming" and "warmup failed/degraded" is not separately surfaced (`PRODUCT_FLOWS.md` Flow 14).

### C-FAIL-003: Unconfigured Data Sources Show Onboarding/Configuration

**Contract**
A module whose data source is not yet configured MUST show an onboarding/configuration row directing the user to set it up, not an empty result.

**Rationale**
- `docs/MODULES.md`: Notes without a root shows "Choose a Notes root folder"; Projects with no roots/matches shows onboarding.
- `MODULE_MATRIX.md` Diagnostic Requirements: Notes onboarding row, Projects onboarding row.
- `docs/QA.md`: "fresh install `n` / `proj` show onboarding rows."

**Applies To**
- Notes (root), Projects (scan roots)
- Any future module with required configuration

**Compliance Signals**
- Notes/Projects module tests; `docs/QA.md` Settings → Notes/Projects manual check.
- Review checkpoint for new modules requiring configuration.

**Current Known Deviations**
- None recorded as an active violation for Notes/Projects. New configuration-dependent modules must add this behavior explicitly (no generic enforcement).

### C-FAIL-004: External-Platform Action Failure Never Reports Success

**Contract**
Built-in platform actions (paste, focus, insert, open URL, window layout, launch, reveal, translate) MUST propagate errors from platform clients and MUST NOT report success when the platform call no-ops. On failure, the panel MUST stay open with a status/diagnostic, per perform-then-dismiss.

**Rationale**
- `docs/ENGINEERING.md`: "Builtin platform actions ... must not report success when the platform call no-ops"; D-015 perform-then-dismiss.
- `PRODUCT_FLOWS.md` Flow 9: `ActionExecutor.run` propagates thrown errors as `.failure`, keeps the panel open; `performActionThenDismiss` failure branch does not dismiss.
- `docs/QA.md` Action feedback: permission-denied action shows status, panel stays open; copy success shows brief status + delayed dismiss.

**Applies To**
- `ActionExecutor.run`, `ActionExecutionFailureMapper`, `LauncherActionFeedback`
- `PasteboardClient`, `AccessibilityClient`, `WorkspaceClient`, `TranslationClient`

**Compliance Signals**
- `swift test` action-failure tests (`ActionFailureFeedbackTests`, `PasteOutcomeTests`, `StabilizationFlowTests`).
- `docs/QA.md` Action feedback manual checks.

**Current Known Deviations**
- The `ActionExecutor`-side propagation contract is confirmed, but whether each individual platform client (`PasteboardClient`/`AccessibilityClient`/`WorkspaceClient`) always throws rather than silently no-ops on every path is unconfirmed (`PRODUCT_FLOWS.md` Flow 9).

### C-FAIL-005: Diagnostic Behavior Is Consistent Across Modules

**Contract**
Diagnostic/failure behavior MUST follow a shared, documented taxonomy (permission-required, degraded/warming, onboarding, timeout, empty-acceptable). Each module's classification MUST be explicit and consistent; modules MUST NOT invent ad-hoc failure semantics that diverge from the taxonomy.

**Rationale**
- `MODULE_MATRIX.md` Classification Legend defines the Failure Behavior taxonomy but records that behavior is applied per-module.
- `PRODUCT_FLOWS.md` Flow 7: disabled module → diagnostic row, but non-`.queryable` module → silent empty; per-module cold rows are bespoke strings vs. the generic `module.warming`.
- Phase 2 fact: "diagnostic behavior is asymmetric" across modules.

**Applies To**
- `ModuleDiagnosticResults`, `PermissionResultBuilder`, `ModuleDiagnostic` kinds
- All modules' failure paths

**Compliance Signals**
- Cross-module `ModuleColdCacheTests`, `ModuleHandleContractTests`.
- Review checkpoint: new failure states map to an existing taxonomy kind.
- A single documented failure-behavior table (in `docs/ENGINEERING.md` or `docs/MODULES.md`) that every module row references.

**Current Known Deviations**
- Non-`.queryable` targeted modules return silent empty (no diagnostic) while disabled modules return a diagnostic row (`PRODUCT_FLOWS.md` Flow 7).
- Some modules return bespoke cold-state strings from their own `handle` while the dispatcher emits a generic `module.warming` row, so two cold-state mechanisms coexist (`PRODUCT_FLOWS.md` Flows 7/14).
- "Empty acceptable" vs "diagnostic required" is decided per-module in `MODULE_MATRIX.md`, not enforced by a shared mechanism.

### C-FAIL-006: Corruption / Read-Failure / Quarantine Is Visible To Doctor/Diagnostics

**Contract**
Configuration/store corruption, read failure, and quarantine outcomes MUST have a defined visibility contract: they MUST be reportable to `cmd doctor` / diagnostics, and the visibility MUST be uniform across the persistence mechanisms the app actually uses.

**Rationale**
- `docs/ENGINEERING.md` / D-018: singleton JSON configs quarantine corrupt JSON; `ConfigCorruptionRegistry` feeds `cmd doctor`.
- `PRODUCT_FLOWS.md` Flow 15: `JSONConfigPersistence` records decode failures to `ConfigCorruptionRegistry`; but `ClipboardHistoryStore` and `JSONFileStore` have separate quarantine paths, and `JSONFileStore` does not feed the registry; read failures (vs decode failures) are silently treated as fallback.
- `MODULE_MATRIX.md` Diagnostic Requirements: three distinct persistence/quarantine paths coexist.

**Applies To**
- `JSONConfigPersistence`, `ConfigCorruptionRegistry`
- `ClipboardHistoryStore`, `JSONFileStore` (Snippets/Media)
- `CommandsModule` doctor surface (`LumaDiagnostics`)

**Compliance Signals**
- `swift test --filter JSONConfigPersistence`, `CommandsModuleDoctorTests`.
- `docs/QA.md`: "`cmd doctor` reports ... corrupt config files."
- Review checkpoint: a new persisted store defines and documents its corruption/read-failure/doctor visibility.

**Current Known Deviations**
- `JSONFileStore` (Snippets, Media) quarantine does not call `ConfigCorruptionRegistry`, so its corruption is not confirmed to appear in `cmd doctor` (`PRODUCT_FLOWS.md` Flow 15).
- `ClipboardHistoryStore` uses a separate `.corrupt-<ts>.bak` scheme independent of the registry.
- `JSONConfigPersistence` treats a present-but-unreadable file (I/O/permission error) as silent fallback, not corruption — never recorded to the registry (`PRODUCT_FLOWS.md` Flow 15).
- `ConfigCorruptionRegistry` is purely in-memory and does not survive restarts (`PRODUCT_FLOWS.md` Flow 15).

---

## E. UI State Ownership Contracts

### C-UI-001: Panel Visibility Has A Single Owner

**Contract**
Panel visibility MUST be owned only by `LauncherWindowController` together with `LauncherPanelVisibilitySession`. All show/hide MUST go through this owner's generation-guarded API.

**Rationale**
- `docs/ENGINEERING.md` Launcher Contract: `LauncherPanelVisibilitySession` backs `LauncherWindowController` show/hide tokens; show/hide generation guards defined.
- `PRODUCT_FLOWS.md` Cross-Cutting State Owners: Panel visibility owner = `LauncherWindowController` + `LauncherPanelVisibilitySession`.
- Workspace rule `launcher-panel-chrome`: `LauncherPanel.position(on:)` is the only placement API.

**Applies To**
- `LauncherWindowController`, `LauncherPanelVisibilitySession`, `LauncherPanel`

**Compliance Signals**
- `swift test --filter LauncherPanelVisibilitySession`, `HotkeyToggle`, `LauncherShowHideStateTests`.
- `scripts/scan_appkit_executor_risk.sh` and `docs/QA.md` show/hide manual smoke.

**Current Known Deviations**
- Menu bar "Show" calls `show()` directly, bypassing `showFromCarbonHotkey()`'s "only when hidden" guard and the 120 ms Carbon-show debounce; this bypass is not documented in `docs/ENGINEERING.md`'s two-paths description (`PRODUCT_FLOWS.md` Flow 3).

### C-UI-002: Query Text Updates Through A Defined Path

**Contract**
Query text MUST flow through a single defined path: `LumaSearchBar` (UI source of truth) → `LauncherRootController` → `LauncherViewModel`. Per-keystroke routing MUST use the `QueryView` snapshot of the live field; the permission banner and routing MUST read the live field, not a stale normalized snapshot.

**Rationale**
- `docs/ENGINEERING.md`: "Per-keystroke routing uses `QueryView` (event snapshot from `searchBar.stringValue`); permission banner routes from the live search field, not a stale normalized snapshot."
- `PRODUCT_FLOWS.md` Cross-Cutting + Flow 5: `searchBar.stringValue` is the live value; `QueryView` is a routed snapshot.
- Workspace rule `launcher-navigation`: detail-mode search field persists suspended queries via `searchBar.persistedQuery`, not `stringValue` alone.

**Applies To**
- `LumaSearchBar`, `LauncherRootController.handleTextChange`, `LauncherViewModel.queryChanged`, `QueryView`

**Compliance Signals**
- `swift test --filter QueryView` (`QueryViewTests`), `PermissionBannerContextTests`.
- `docs/QA.md`: "Permission banner follows live `searchBar.stringValue`, not a stale normalized snapshot."

**Current Known Deviations**
- None recorded as an active violation. Detail-mode clears the visible field while suspending the query, which requires readers to use `persistedQuery`; this is the documented contract, not a deviation.

### C-UI-003: Content Mode Has A Single Owner

**Contract**
The home/results/detail content mode MUST be owned by `LauncherContentCoordinator` as the single source (`LauncherContentMode`). No other object MUST independently decide the presentation mode.

**Rationale**
- `docs/ENGINEERING.md`: `LauncherContentMode` enum in `LauncherKeyRouter.swift`; `LauncherContentCoordinator` holds the runtime `mode` value and is the single owner for home/results/detail presentation.
- `PRODUCT_FLOWS.md` Cross-Cutting State Owners: content mode owner = `LauncherContentCoordinator`.

**Applies To**
- `LauncherContentCoordinator`, `LauncherContentMode`

**Compliance Signals**
- `swift test --filter LauncherSession`, detail/home tests.
- Review checkpoint: mode transitions route through the coordinator.

**Current Known Deviations**
- None recorded. Enum type location documented in `docs/ENGINEERING.md` (P3.1); runtime ownership remains `LauncherContentCoordinator`.

### C-UI-004: Selection State Has A Defined Bridge

**Contract**
Selection state MUST be maintained by `LauncherListView` (mouse/keyboard-driven source) bridged to `LauncherContentCoordinator.selectedIndex` through the defined `onSelectionChanged` path. Action-panel selection and list selection are separate states with defined priority.

**Rationale**
- `PRODUCT_FLOWS.md` Cross-Cutting + Flow 8: list view is the source; `onSelectionChanged` pushes the index into the coordinator; action-panel selection has priority when visible.
- Workspace rule `launcher-navigation`: action-panel arrow keys work regardless of whether search or list holds focus.

**Applies To**
- `LauncherListView`, `LauncherContentCoordinator`, `LauncherActionPanel`, `LauncherKeyboardDispatcher`

**Compliance Signals**
- `swift test --filter LauncherListRowReuse`, `LauncherSnapshot`.
- `docs/QA.md`: "Command+1...9 targets visible result/action rows."

**Current Known Deviations**
- Stale-selection fallback to index 0 after a snapshot removes the previously selected item is a structural possibility (a Return could run a different row than last shown) — recorded as an observation, not an isolated defect (`PRODUCT_FLOWS.md` Flow 8).

### C-UI-005: Detail Lifecycle Uses Defined Entry/Exit Paths

**Contract**
Detail presentation MUST enter and exit only through the defined path: `LauncherDetailPresenter` / `LauncherDetailLifecycleController` / `ModuleDetailRegistry` / `LauncherContentCoordinator`. User-facing exit MUST go through `exitDetailFromChrome()`; `beginDetailMode`/`endDetailMode`/`cancelDetailMode` MUST stay paired and always restore `isEditable`.

**Rationale**
- Workspace rule `launcher-navigation`: user-facing exit → `exitDetailFromChrome()`, not `closeDetail()` alone; begin/end/cancel paired; every exit restores editability.
- `docs/ENGINEERING.md` D-008/D-021: detail stays in-panel with preserved keyboard exit/editability; presentation/keyboard routing in `LauncherDetailPresenter`/`LauncherKeyboardDispatcher`.
- `PRODUCT_FLOWS.md` Cross-Cutting: four-way detail lifecycle split.

**Applies To**
- `LauncherDetailPresenter`, `LauncherDetailLifecycleController`, `ModuleDetailRegistry`, `LauncherContentCoordinator`, `LauncherSearchDetailMode`

**Compliance Signals**
- `swift test --filter DetailHierarchy`, `LauncherSearchDetailMode`, `LauncherDetailExitPlanner`.
- `docs/QA.md`: "Search field remains editable after leaving detail."

**Current Known Deviations**
- None recorded as an active violation; this flow is among the more test-covered (`PRODUCT_FLOWS.md` Flow 11). Typing-to-exit uses `cancelDetailMode` + `closeDetail` outside `exitDetailFromChrome`, which is the documented typing path, not a deviation.

### C-UI-006: No Bypassing An Owner To Mutate Shared UI State

**Contract**
No object MUST bypass a defined owner (C-UI-001..005) to directly mutate panel visibility, query text, content mode, selection, or detail state. New code MUST route through the owner's API.

**Rationale**
- The owner model in `docs/ENGINEERING.md` and `PRODUCT_FLOWS.md` Cross-Cutting State Owners exists precisely to prevent multiple writers to the same UI state.
- Phase 0 subjective report ("越来越乱") and the three `.ips` crashes motivate a strict single-writer discipline for UI state.

**Applies To**
- All `LumaApp` launcher code touching shared UI state

**Compliance Signals**
- Review checkpoint: a new direct mutation of a shared UI-state field outside its owner is a violation.
- Executor-boundary/state tests (`LauncherShowHideStateTests`, `HideDuringSnapshotApplyTests`).

**Current Known Deviations**
- Menu bar "Show" bypassing `LauncherWindowController`'s Carbon-show guard/debounce (see C-UI-001) is the one confirmed owner-bypass for visibility (`PRODUCT_FLOWS.md` Flow 3).

---

## F. AppKit / MainActor / Async Contracts

### C-APPKIT-001: AppKit Overrides Follow Swift 6 Executor-Boundary Rules

**Contract**
AppKit `NSView`/`NSPanel`/`NSWindow`/`NSControl`/`NSViewController` subclasses and project wrappers MUST NOT be annotated `@MainActor`; they MUST use `@preconcurrency import AppKit` and mark overrides `nonisolated override`, per `docs/swift6-appkit-boundaries.md`.

**Rationale**
- `docs/swift6-appkit-boundaries.md` (Accepted ADR): full executor-boundary contract; violating it triggers `EXC_BAD_ACCESS`/`_dispatch_assert_queue_fail`.
- Phase 0: three `.ips` crash reports (`SIGSEGV`/`SIGABRT`) on 2026-07-06 — the class of crash this ADR targets (`CURRENT_STATE.md`).

**Applies To**
- All AppKit subclass files in `LumaApp` (`LauncherPanel`, `LumaWindow`, `LauncherListView`, search bar, detail views)

**Compliance Signals**
- `scripts/scan_appkit_executor_risk.sh` exits 0.
- `swift test --filter AppKitExecutor`, `LauncherPanelExecutor`; `LumaLinterTests`.
- `docs/QA.md` Swift 6/AppKit manual smoke (clear `.ips`, then show/hide, Esc, detail, light/dark).

**Current Known Deviations**
- The working tree has uncommitted edits to `scripts/scan_appkit_executor_risk.sh` (the scanner itself) and `LauncherListView.swift` of unknown origin (`CURRENT_STATE.md`); the scanner's current pass/fail state after those edits is not established in Phase 0.

### C-APPKIT-002: C/Carbon/AppKit Callbacks Use Nonisolated Entry + MainActor Bridge

**Contract**
C/Carbon/AppKit callbacks (hotkey handler, Objective-C target/action, `NotificationCenter` observers) MUST use a `nonisolated` entry point and bridge to MainActor via `Task { @MainActor in ... }`. `MainActor.assumeIsolated` and selector-based `NotificationCenter` observers MUST NOT be used; use block-based `LumaNotificationCenter.observe`.

**Rationale**
- `docs/swift6-appkit-boundaries.md` items 6, 8: Carbon/C callbacks nonisolated + `Task { @MainActor }`; block-based observers only.
- `PRODUCT_FLOWS.md` Flow 2: `lumaHotKeyEventHandler` (nonisolated) → `schedulePress()` → `Task { @MainActor onPress() }` implements this.

**Applies To**
- `HotkeyController`, all `@objc` target/action entry points, all `NotificationCenter` observers on MainActor UI

**Compliance Signals**
- `scripts/scan_appkit_executor_risk.sh` (warn-only `@objc` in `@MainActor` types, multiline selector observers).
- `swift test --filter HotkeyToggle` (`HotkeyToggleExecutorTests`), `AppKitExecutor`.

**Current Known Deviations**
- None recorded as an active violation for the hotkey path. The menu-tree Task and some observers' cancellation-on-hide behavior is unconfirmed (`PRODUCT_FLOWS.md` Flow 3) — a C-ASYNC-001 concern rather than a boundary-pattern violation.

### C-APPKIT-003: `Task { @MainActor }` Is A Boundary Bridge, Not A State-Fix Hammer

**Contract**
`Task { @MainActor in ... }` MUST be used only for legitimate boundary bridging (from nonisolated callbacks or to hop back to the main actor after async work). It MUST NOT be used to paper over arbitrary state-ordering bugs or to escape a defined ownership/generation guard.

**Rationale**
- `docs/swift6-appkit-boundaries.md` item 4: use `Task { @MainActor in ... }` when an override needs MainActor state — i.e. specifically at boundaries.
- `docs/ENGINEERING.md` show/hide generation guards and `PRODUCT_FLOWS.md` Flows 3/12 show that async main-actor hops are paired with generation checks (`shouldCompleteHide`, `shouldCompleteDeferredShow`) rather than fire-and-forget state writes.

**Applies To**
- All `Task { @MainActor }` usages in `LumaApp` launcher code

**Compliance Signals**
- Review checkpoint: a new `Task { @MainActor }` that mutates shared UI state without a generation/ownership guard is a violation.
- `swift test --filter CancellationGeneration`, `LauncherPanelVisibilitySession`.

**Current Known Deviations**
- Not separately audited across the whole app in Phase 0-3; flagged as a review lens rather than a confirmed violation site.

### C-ASYNC-001: Every Task Has An Owner, Cancellation, And Generation/Lifecycle Guard

**Contract**
Every `Task` MUST have an identifiable owner, a cancellation point (or a bounded self-terminating lifetime), and — where it applies async results to shared state — a generation guard or lifecycle boundary that rejects stale applies.

**Rationale**
- `docs/ENGINEERING.md`: "Query tasks are cancelled on every new keystroke"; `cancelLauncherAsyncWork()` cancels in-flight query/snapshot/home/permission/workbench/detail tasks; `CancellationGeneration` backs restore.
- `PRODUCT_FLOWS.md` Flows 3/6/12/14: generation guards, `revalidateTask` cancellation, `LauncherTaskRegistry`, idle-teardown scheduling.

**Applies To**
- `QueryDispatcher`, `LauncherViewModel`, `LauncherRootController`, `LauncherWindowController`, `PermissionBannerController`, module tasks

**Compliance Signals**
- `swift test --filter CancellationGeneration`, `RevalidateCancellation`, `LauncherPanelVisibilitySession`, `NotesDetailRefreshCancellation`.
- Review checkpoint: new Tasks show owner + cancellation.

**Current Known Deviations**
- Whether the `LauncherViewModel`-owned query dispatch `Task` is actually cancelled on every new keystroke is documented but not independently re-confirmed as code (`PRODUCT_FLOWS.md` Flow 6).
- The show-path menu-tree Task's self-cancellation on hide is unconfirmed (`PRODUCT_FLOWS.md` Flow 3).
- `AppCoordinator`-level idle/memory-pressure teardown scheduling has no located dedicated test (`PRODUCT_FLOWS.md` Flow 14).

### C-ASYNC-002: Show/Hide/Detail/Restore/Snapshot-Apply Use Generation Guards

**Contract**
Show, hide, detail presentation, session restore, and snapshot apply MUST use generation/cancellation guards so that a stale async completion cannot apply after a superseding action.

**Rationale**
- `docs/ENGINEERING.md` show/hide generation guards: `shouldCompleteHide`, `shouldCompleteDeferredShow`, `CancellationGeneration`, `LauncherSnapshotApplyCoalescer.cancel()` on hide, `LauncherSnapshotApplyPolicy` gating.
- `PRODUCT_FLOWS.md` Flows 3/10/12: deferred-show guard, detail presentation-generation, `finishHide` `shouldCompleteHide`.

**Applies To**
- `LauncherPanelVisibilitySession`, `CancellationGeneration`, `LauncherDetailLifecycleController`, `LauncherSnapshotApplyPolicy`/`Coalescer`, restore path

**Compliance Signals**
- `swift test --filter LauncherPanelVisibilitySession`, `CancellationGeneration`, `LauncherSnapshotApplyPolicy`, `HideDuringSnapshotApply`, `LauncherSnapshotApplyCoalesce`.
- `docs/QA.md`: rapid toggle ×50, hide-during-cross-fade checks.

**Current Known Deviations**
- The interrupted-crossfade fallback on hide is documented but not independently re-verified as code in Phase 3 (`PRODUCT_FLOWS.md` Flow 11).

### C-ASYNC-003: Background Refresh Does Not Repaint Hidden Panel Or Block Hotkey→Visible

**Contract**
Background refresh (Open Apps, caches, warmup) MUST NOT repaint a hidden/visible panel's home list mid-interaction, MUST NOT grow refresh counters while the panel is hidden, and MUST NOT block the hotkey→visible path.

**Rationale**
- `docs/ENGINEERING.md` Performance Contract: "Background Open Apps cache changes must not repaint the visible home list while the panel is open"; "Open Apps refresh is bound to panel visibility; hidden panel must not grow `openApps.refresh` counters"; "Showing the launcher must not rebuild Open Apps for the first visible frame."
- Phase 0: hotkey p95 ≈ 8.3 s — the strongest signal that something blocks or delays the show/first-frame path (`CURRENT_STATE.md`, `PRODUCT_FLOWS.md` Flows 3/4).

**Applies To**
- `OpenAppsHomeProvider`, `LauncherHomeCoordinator`, `ClipboardPasteboardCache`, module background refresh tasks
- Show path (`LauncherWindowController.show`) and first-frame home rendering

**Compliance Signals**
- `swift test --filter BackHome`, `PanelSignals`, `LauncherSnapshot`.
- `docs/QA.md`: "First frame shows empty home without rebuilding Open Apps"; "Panel hide with Open Apps hidden does not trigger extra Open Apps refresh work."
- `latency-report.json` hotkey/keystroke p95 within RC ceilings when checked (`docs/QA.md` § Performance Gate: hotkey ≤ 1000 ms emergency, keystroke ≤ 60 ms hard ceiling via `LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh`). Engineering targets 50/80 ms are aspirational (P3.3).

**Current Known Deviations**
- Phase 0 hotkey p95 ≈ 8.3 s is ~100× the documented ceiling — a standing violation of the hotkey→visible budget whose root cause is deliberately not diagnosed in Phase 0-3.

---

## G. Detail Contracts

### C-DETAIL-001: Detail Views Do Not Drive Module Lifecycle In Reverse

**Contract**
A detail view MUST NOT drive module registration/warmup/teardown in reverse. Detail views read shared module instances through `ModuleDetailRegistry`; modules MUST NOT import or call the registry, and detail views MUST NOT own module lifecycle.

**Rationale**
- `docs/ENGINEERING.md`: "Module detail views read shared module instances through `ModuleDetailRegistry` in `LumaApp`. Modules do not import or call the registry."
- `docs/DECISIONS.md` D-021: presentation/keyboard routing live in `LauncherDetailPresenter`/`LauncherKeyboardDispatcher`.

**Applies To**
- All detail views, `ModuleDetailRegistry`, `ModuleHost`

**Compliance Signals**
- Import audit: detail views do not mutate `ModuleHost` warmup/enabled state directly.
- `swift test --filter DetailHierarchy`.

**Current Known Deviations**
- Wordbook review and some flows run inside detail (`WordbookModule.perform` throws `unsupportedAction`), so detail drives the review action rather than `perform`; noted as fact (`MODULE_MATRIX.md`) — bounded to action execution, not lifecycle.

### C-DETAIL-002: Detail Open Goes Through Presenter/Registry/Coordinator

**Contract**
Opening a detail MUST go through `LauncherDetailPresenter` → `ModuleDetailRegistry.makeDetailView` → `LauncherContentCoordinator.present`, with the module-enabled check applied before presentation.

**Rationale**
- `PRODUCT_FLOWS.md` Flow 10: `openModuleDetail` → `presentModuleDetail`; `isModuleEnabledForDetail` guards; registry pool + generation; coordinator sets `mode = .detail`.
- `docs/DECISIONS.md` D-021.

**Applies To**
- `LauncherDetailPresenter`, `ModuleDetailRegistry`, `LauncherContentCoordinator`

**Compliance Signals**
- `swift test --filter DetailHierarchy`, `LauncherSearchDetailMode`.
- Review checkpoint: no ad-hoc detail attach bypassing the presenter.

**Current Known Deviations**
- None recorded as an active violation.

### C-DETAIL-003: Detail Reuse/Pooling Keeps A Generation/Content-Revision Contract

**Contract**
Pooled/reused detail views MUST preserve a generation/content-revision guard so an unchanged content generation skips redundant reloads and a stale async refresh cannot write after deactivate/hide/close.

**Rationale**
- `docs/ENGINEERING.md`: "Detail views reuse pooled instances; `activate(generation:)` skips redundant reloads when content generation is unchanged"; Notes uses `NotesDetailRefreshGate`.
- `PRODUCT_FLOWS.md` Flow 10: `refreshDetailContentGeneration`, `lastActivatedGeneration` skip logic.

**Applies To**
- `ModuleDetailRegistry` (`detailPool`, `lastActivatedGeneration`), `NotesDetailRefreshGate`

**Compliance Signals**
- `swift test --filter DetailHierarchy` (`detailRegistrySkipsRedundantActivationWhenGenerationUnchanged`), `NotesDetailRefreshCancellation`.
- `docs/QA.md`: "`detail.viewMade` stays flat on second open."

**Current Known Deviations**
- None recorded as an active violation.

### C-DETAIL-004: Detail Exit Goes Through The Exit Planner

**Contract**
Returning from detail MUST go through `LauncherDetailExitPlanner` via `exitDetailFromChrome()`, distinguishing restore-suspended-query, return-to-home, and typing-cancel outcomes. Every exit MUST restore search editability.

**Rationale**
- Workspace rule `launcher-navigation`: user-facing exit → `exitDetailFromChrome()`; `closeDetail()` is teardown only.
- `PRODUCT_FLOWS.md` Flow 11: `LauncherDetailExitPlanner.outcome(...)` → `.reenableSearchOnly` / `.restoreSuspendedQuery` / `.returnToHome`; typing exit uses `cancelDetailMode`.

**Applies To**
- `LauncherDetailExitPlanner`, `LauncherRootController.exitDetailFromChrome`, `LauncherSearchDetailMode`

**Compliance Signals**
- `swift test --filter LauncherDetailExitPlanner`, `DetailTypingEscapeConsistency`.
- `docs/QA.md`: Esc-from-detail returns home in one frame; typing-then-Esc goes home without restoring the suspended prefix.

**Current Known Deviations**
- None recorded as an active violation.

### C-DETAIL-005: Modules Without A Registered Detail Do Not Advertise Open-Detail

**Contract**
A module without a registered detail view MUST NOT expose an open-detail behavior unless it provides a clear alternative user result. `bareBehavior` and result actions MUST match the module's actual detail registration.

**Rationale**
- `MODULE_MATRIX.md` Summary Matrix "Detail View": Apps, Commands, Menu Items, Kill Process, Browser Tabs, Window Layouts, Windows have no registered detail; those modules use non-detail bare behaviors.
- `PRODUCT_FLOWS.md` Flow 10: `makeDetailView` returns `nil` when no factory is registered; whether an open-detail path is reachable for these modules is unconfirmed.

**Applies To**
- `ModuleDetailRegistry.makeDefault()` registrations vs each module's `bareBehavior`/actions

**Compliance Signals**
- Review checkpoint: a module advertising `.openDetail` has a registered detail factory.
- Cross-check `MODULE_MATRIX.md` Detail View column against `bareBehavior`.

**Current Known Deviations**
- Whether any no-detail module's open-detail path is reachable via normal user flow is unconfirmed (`PRODUCT_FLOWS.md` Flow 10). Most such modules have no `.openDetail` bare behavior, so this is likely unreachable but not proven.

---

## H. Cache / Persistence / Diagnostics Contracts

### C-CACHE-001: Every Cache Has Owner, Scope, And Invalidation

**Contract**
Every cache MUST have a defined owner, scope, and TTL or invalidation trigger. A cache MUST NOT retain data past its documented lifetime or leak across module generations.

**Rationale**
- `docs/ENGINEERING.md` Performance Contract: stale-while-revalidate default; `QuerySnapshotCache` TTL/LRU; per-module TTLs (running refresh 2s, memory-top 2s, process list 3s, AX trust 5s).
- `MODULE_MATRIX.md` per-module cache/warmup notes; `QuerySnapshotCache` keyed by `moduleGeneration|normalizedQuery`, TTL 300 s, max 64 LRU.

**Applies To**
- `QuerySnapshotCache`, `UsageResultCache`, `PanelSignalsCache`, per-module caches, `IconCache`

**Compliance Signals**
- `swift test --filter QuerySnapshot`, `PanelSignals`, per-module cache tests (`AppsMemoryTopSWRTests`).
- Review checkpoint: a new cache documents owner/scope/invalidation.

**Current Known Deviations**
- `clipboard-history.json` observed ~38 MB with a `.corrupt-<ts>.bak` sibling (`MODULE_MATRIX.md`) — retention bounding is a Clipboard-specific concern, noted as fact.

### C-CACHE-002: Caches Never Hide Permission/Corruption/Missing-Source

**Contract**
A cache (including stale-while-revalidate) MUST NOT mask a permission failure, config corruption, or missing data source. A stale cached result MUST NOT be presented in a way that suppresses a required diagnostic.

**Rationale**
- `docs/ENGINEERING.md`: cold caches show warming/degraded rows; permission-blocked modules return diagnostics.
- `PRODUCT_FLOWS.md` Flow 15: corruption is invisible at the moment it happens (module proceeds with fallback), only visible via `cmd doctor` — a case where the fallback/cache path hides the failure from the immediate surface.

**Applies To**
- All warm-cache modules, `QuerySnapshotCache`, home providers

**Compliance Signals**
- `swift test --filter ModuleColdCache`, `QuerySnapshot`.
- Review checkpoint: a cache-hit path still surfaces required permission/corruption diagnostics.

**Current Known Deviations**
- Config corruption produces no immediate UI signal; the module proceeds with `fallback` as if the file never existed (`PRODUCT_FLOWS.md` Flow 15) — the fallback path hides the failure until `cmd doctor`.

### C-PERSIST-001: Every Persistence Format Defines Corruption/Read-Failure Behavior

**Contract**
Every persistence format MUST define and document its behavior for corruption (decode failure) and read failure (I/O/permission), including whether it quarantines, records to the corruption registry, and surfaces to doctor.

**Rationale**
- `docs/DECISIONS.md` D-018 + `docs/ENGINEERING.md`: `JSONConfigPersistence` quarantines on decode failure; doctor lists corrupt files.
- `PRODUCT_FLOWS.md` Flow 15 + `MODULE_MATRIX.md`: three distinct persistence/quarantine paths (`JSONConfigPersistence`, `ClipboardHistoryStore`, `JSONFileStore`) with different behavior; read failures treated as silent fallback.

**Applies To**
- `JSONConfigPersistence`, `JSONFileStore`, `ClipboardHistoryStore`, EventKit/Keychain-backed stores

**Compliance Signals**
- `swift test --filter JSONConfigPersistence` (load + save).
- Review checkpoint: a new persisted format documents corruption/read-failure behavior.

**Current Known Deviations**
- `JSONConfigPersistence` treats present-but-unreadable files as silent fallback (only decode failures quarantine) (`PRODUCT_FLOWS.md` Flow 15).
- `JSONFileStore` and `ClipboardHistoryStore` have separate quarantine paths; `JSONFileStore` does not record to `ConfigCorruptionRegistry`.

### C-PERSIST-002: Doctor/Diagnostics Can Report Key Config/Store Corruption

**Contract**
`cmd doctor` / diagnostics MUST be able to report corruption of the app's key config/stores across the persistence mechanisms actually in use — not only the subset handled by one path.

**Rationale**
- `docs/ENGINEERING.md`: "`ConfigCorruptionRegistry` feeds `cmd doctor`"; `docs/QA.md`: doctor reports corrupt config files.
- `PRODUCT_FLOWS.md` Flow 15: `cmd doctor`'s corrupt-file list is sourced solely from `ConfigCorruptionRegistry`, which only reflects `JSONConfigPersistence` users; `ConfigCorruptionRegistry` is in-memory (no restart persistence).

**Applies To**
- `ConfigCorruptionRegistry`, `CommandsModule` doctor, `LumaDiagnostics`

**Compliance Signals**
- `swift test --filter CommandsModuleDoctor`, `JSONConfigPersistence`.
- `docs/QA.md`: "`cmd doctor` reports ... corrupt config files."

**Current Known Deviations**
- `cmd doctor` corrupt-file list is incomplete: it excludes `JSONFileStore` (Snippets/Media) and `ClipboardHistoryStore` corruption (`PRODUCT_FLOWS.md` Flow 15).
- `ConfigCorruptionRegistry` is in-memory only; corruption from a prior run is not shown after restart (`PRODUCT_FLOWS.md` Flow 15).

### C-DIAG-001: Diagnostics Export Is Reachable, Stable, Redacted, Local-Only

**Contract**
Diagnostics export MUST be reachable by the user, produce a stable redacted local-only artifact, and never transmit off-device. Its entry point MUST NOT be fully cut off by the default configuration without a documented alternative.

**Rationale**
- `docs/ENGINEERING.md` / `docs/PERMISSIONS.md`: menu bar **Run Doctor…** / **Export Diagnostics…** via `AppHostService`; `DiagnosticsExport` writes redacted JSON to `~/Library/Logs/Luma/diagnostics.json`; local-only, no telemetry.
- `docs/QA.md` / `P2_EXIT_SUMMARY.md`: `./scripts/run_p0_smokes.sh` (`LUMA_QA_EXPORT=1`) validates export on the signed app.
- *(Phase 0 historical evidence only: `cmd export-diagnostics` was Commands-gated and `diagnostics.json` was missing before P0.8 recovery wiring — `CURRENT_STATE.md`, `PRODUCT_FLOWS.md` Flow 16.)*

**Applies To**
- `CommandsModule` `export-diagnostics`, `AppHostService.exportDiagnostics`, `DiagnosticsExport`

**Compliance Signals**
- `swift test --filter DiagnosticsExportRedaction`, `CrashLogRedactionAtWrite`.
- `docs/QA.md`: `cmd export-diagnostics` writes redacted JSON.
- Reachability review: export entry is not fully blocked on a fresh install.

**Current Known Deviations**
- ~~`cmd export-diagnostics` unreachable on fresh install~~ — **Resolved P0.8/P3.1:** menu bar **Export Diagnostics…** and `LUMA_QA_EXPORT` smoke (`P2_EXIT_SUMMARY.md`) validate `diagnostics.json` on default install. Mirrored `cmd export-diagnostics` when Commands is enabled.

### C-DIAG-002: Declared Diagnostics Fields Have Real Population Sources

**Contract**
Every field declared in the diagnostics payload / documented in the docs MUST have a real population source at the production call site. Fields MUST NOT be documented as populated while shipping empty/`null` long-term.

**Rationale**
- `docs/ENGINEERING.md` / `docs/PERMISSIONS.md`: payload sections `platform`, `modules`, `permissions`, `recentErrors`, `corruptConfigFiles`, `crashLogPath`, `crashLogWriteStatus`.
- `RecoveryDiagnosticsCollector.buildExportPayload` (via `AppHostService.exportDiagnostics`) populates all declared sections at the production call site (P0.8/P2.5).
- *(Phase 0 historical evidence only: pre-P0.8 call site passed only `latencyP95` + breadcrumbs — `PRODUCT_FLOWS.md` Flow 16, `MODULE_MATRIX.md`.)*

**Applies To**
- `DiagnosticsExport.buildPayload`, `AppHostService.exportDiagnostics`, `DiagnosticsPayload`

**Compliance Signals**
- `swift test --filter DiagnosticsExportRedaction` extended to assert non-empty declared fields.
- Doc check: `docs/ENGINEERING.md` payload description matches actually-populated fields.

**Current Known Deviations**
- ~~`platform`/`modules`/`permissions`/`recentErrors` un-populated at production call site~~ — **Resolved P0.8/P2.5:** `RecoveryDiagnosticsCollector.buildExportPayload` populates all declared sections; validated by `LUMA_QA_EXPORT` in `./scripts/run_p0_smokes.sh` (`P2_EXIT_SUMMARY.md`).

### C-DIAG-003: Crash-Breadcrumb Write Failure Is Isolated But Explainable

**Contract**
Crash-breadcrumb writes MUST NOT affect the app's main path on failure, but a persistent write failure MUST NOT be entirely unexplainable — the failure mode MUST be documented (and ideally observable) rather than silently swallowed with no defined behavior.

**Rationale**
- `docs/ENGINEERING.md` / `docs/PERMISSIONS.md`: `crash-log.txt` at `~/Library/Application Support/Luma/crash-log.txt`; export includes `crashLogPath` and `crashLogWriteStatus`.
- `CrashLogBuffer.persist()` uses `do`/`catch`, sets `lastPersistFailed`, and exposes status via diagnostics export.
- *(Phase 0/3 historical evidence only: earlier `persist()` used silent `try?` — `PRODUCT_FLOWS.md` Flow 16.)*

**Applies To**
- `CrashLogBuffer`, `CrashLogRecording`

**Compliance Signals**
- `swift test --filter CrashLogRedactionAtWrite`.
- Doc check: `crash-log.txt` write-failure behavior documented in `docs/ENGINEERING.md`/`docs/PERMISSIONS.md`.

**Current Known Deviations**
- `CrashLogBuffer.persist()` records `lastPersistFailed` and exposes `crashLogWriteStatus` in export, but there is no in-app alert if the file stops updating and the user never exports diagnostics.

### C-DIAG-004: Diagnostics And Crash-Log Paths Are Documented And Consistent

**Contract**
The on-disk locations of `diagnostics.json` and `crash-log.txt` MUST be documented consistently across docs, and the docs MUST match the code's actual write locations.

**Rationale**
- `docs/PERMISSIONS.md` / `docs/ENGINEERING.md`: `diagnostics.json` → `~/Library/Logs/Luma/diagnostics.json`; `crash-log.txt` → `~/Library/Application Support/Luma/crash-log.txt` (both documented P3.1).
- *(Phase 0 historical evidence only: docs omitted `crash-log.txt` and Phase 0 checked `~/Library/Logs/Luma/` — `CURRENT_STATE.md`, `MODULE_MATRIX.md`.)*

**Applies To**
- `DiagnosticsExport.exportToLogsDirectory`, `CrashLogBuffer`, `docs/PERMISSIONS.md`, `docs/ENGINEERING.md`

**Compliance Signals**
- Doc check: both paths documented and matching code.
- Review checkpoint on any change to diagnostics/crash-log paths.

**Current Known Deviations**
- None recorded for path documentation (P3.1). `CrashLogBuffer.persist()` sets `lastPersistFailed` without in-app alert if export is never run — see C-DIAG-003.

---

## I. Defaults / Product Surface Contracts

### C-DEFAULT-001: Default-On Set Is Few And Stable

**Contract**
The default-on module set MUST be small and stable, matching the MVP decision. Expanding it requires a new `docs/DECISIONS.md` entry.

**Rationale**
- `docs/DECISIONS.md` D-012: seven default-on modules.
- `docs/MODULES.md` MVP default-on list.

**Applies To**
- `ModuleWarmupDefaults.defaultEnabledModuleIDs`, manifests, schema v2 migration

**Compliance Signals**
- `swift test --filter BuiltInModules`, migration tests.
- `docs/DECISIONS.md` D-012 matches manifests.

**Current Known Deviations**
- None recorded (P2.1 aligned `docs/PERMISSIONS.md` with D-012).

### C-DEFAULT-002: Default-On Modules Are Useful Without High-Sensitivity Permissions

**Contract**
A default-on module MUST provide a useful experience without requiring high-sensitivity permissions (Automation prompts, screen recording) at first use. Permission prompts MUST be lazy and surface-gated.

**Rationale**
- `docs/ENGINEERING.md`: "Accessibility permission is lazy"; "Browser Tabs is default-off because AppleScript and Automation prompts are sensitive."
- `docs/DECISIONS.md` D-010: accessibility guidance lazy and path-specific.
- `MODULE_MATRIX.md`: default-on modules' search paths need no permission (Apps search, Clipboard search, Snippets copy, Quicklinks); AX is only for specific actions.

**Applies To**
- All default-on modules
- `AccessibilityGuidancePolicy`, `PermissionBannerController`

**Compliance Signals**
- `docs/QA.md` Permissions: no AX banner on empty home/ordinary search when denied.
- `swift test --filter PermissionBanner`.

**Current Known Deviations**
- None recorded as an active violation for the D-012 default-on set. Todo (default-on) requires EventKit but degrades to an actionable permission row rather than prompting on plain search (`MODULE_MATRIX.md`) — the documented behavior.

### C-DEFAULT-003: High-Permission/Expert/Heavy Modules Are Default-Off Or Deferred

**Contract**
Modules that require high-sensitivity permissions, are expert-only, carry heavy caches, or easily trigger system authorization prompts MUST default off or be deferred.

**Rationale**
- `docs/DECISIONS.md` D-012/D-020: expert modules default off; Window Layouts default off until warm-cache ships.
- `MODULE_MATRIX.md` Default-Off/Deferred candidates (Commands, Media, Browser Tabs, Menu Items, Window Layouts, Wordbook, Secrets, Kill Process, Projects; Windows deferred).

**Applies To**
- `ModuleWarmupDefaults.expertDefaultOffModuleIDs`, manifests

**Compliance Signals**
- `swift test --filter BuiltInModules`.
- `docs/DECISIONS.md`/`docs/MODULES.md` default-off list matches manifests.

**Current Known Deviations**
- None recorded for the D-012 default-off set. `docs/PERMISSIONS.md` aligned P2.1.

### C-DEFAULT-004: Defaults Are Consistent Across Docs, Manifest, And Warmup Defaults

**Contract**
A module's default state MUST be identical across `docs/PERMISSIONS.md`, `docs/MODULES.md`, its manifest `defaultEnabled`, and `ModuleWarmupDefaults`. Module display names MUST also be consistent across docs.

**Rationale**
- `docs/PERMISSIONS.md`, `docs/MODULES.md`, manifests, and `ModuleWarmupDefaults` agree post-P2.1 (C-DEFAULT-004 resolved).
- *(Phase 0/4 historical evidence only: `MODULE_MATRIX.md` recorded PERMISSIONS Default-column staleness and "Menu Items" vs "Menu Bar Search" naming — corrected P2.1.)*

**Applies To**
- `docs/PERMISSIONS.md`, `docs/MODULES.md`, `docs/DECISIONS.md`, all manifests, `WarmupTier.swift`

**Compliance Signals**
- Doc check comparing the four sources for each module.
- `swift test --filter BuiltInModules` for the manifest/warmup-defaults side.

**Current Known Deviations**
- None recorded (P2.1 resolved PERMISSIONS defaults and Windows manifest metadata).

### C-DEFAULT-005: Diagnostics/Recovery Entry Is Not Fully Cut Off By Default Config

**Contract**
Diagnostic and recovery entry points (doctor, export-diagnostics) MUST NOT be entirely unreachable under the default configuration. If they are gated behind a default-off module, there MUST be a documented alternative entry or the gating MUST be recorded as an explicit, accepted fact.

**Rationale**
- `docs/ENGINEERING.md` / `docs/MODULES.md`: menu bar **Run Doctor…** / **Export Diagnostics…** reach `AppHostService` on a default install (Commands off).
- `docs/QA.md`: P0 gate includes `LUMA_QA_EXPORT` via `./scripts/run_p0_smokes.sh`.
- *(Phase 0 historical evidence only: bare `cmd doctor` / `cmd export-diagnostics` were Commands-gated and `diagnostics.json` was absent — `CURRENT_STATE.md`, `PRODUCT_FLOWS.md` Flow 16.)*

**Applies To**
- `CommandsModule` (doctor, export-diagnostics), Settings, menu bar
- `docs/DECISIONS.md`, `docs/ENGINEERING.md`

**Compliance Signals**
- Reachability review: doctor/export reachable on a fresh install, or a documented alternative exists.
- `docs/QA.md` release checklist covers a diagnostics path.

**Current Known Deviations**
- None recorded. Menu bar recovery entry satisfies the contract on default install (P0.8/P3.1).

---

## J. Testing / Review Contracts

### C-TEST-001: Every P0 Flow Has An Automated Test Or Explicit Manual QA

**Contract**
Every P0/main-path flow in `PRODUCT_FLOWS.md` MUST be covered by an automated test or an explicit manual QA checklist item. A P0 flow MUST NOT be left with neither.

**Rationale**
- `PRODUCT_FLOWS.md` enumerates 16 flows with per-flow Test Coverage and Gap sections; several gaps are flagged (startup end-to-end, hotkey failure branch, latency wall-clock).
- `docs/QA.md` provides the manual smoke and automated-gate lists.

**Applies To**
- All 16 flows in `PRODUCT_FLOWS.md`
- `docs/QA.md`, `Tests/**`

**Compliance Signals**
- `swift test`; targeted filters in `docs/QA.md`.
- Coverage review mapping each flow to a test or a QA checklist item.

**Current Known Deviations**
- No located end-to-end test for `AppCoordinator.start()` startup, hotkey registration-failure branch, `AppCoordinator` idle-teardown scheduling, or wall-clock hotkey latency (`PRODUCT_FLOWS.md` Flows 1/2/14, Gaps).
- `launcherFlowHarnessReplaysQuery` currently fails (`CURRENT_STATE.md`).

### C-TEST-002: Every Module Has Handle-Contract Coverage

**Contract**
Every module MUST have handle-contract test coverage proving: `handle` does not block on the hot path, permission failure is visible, and cold cache is visible.

**Rationale**
- `docs/ENGINEERING.md` Query And Module Contract; `MODULE_MATRIX.md` Hot Path And Blocking Risk notes coverage is partial and per-module.
- `Tests/LumaModulesTests/ModuleHandleContractTests.swift`, `ModuleColdCacheTests.swift` exist as proxies.

**Applies To**
- All modules; `ModuleHandleContractTests`, `ModuleColdCacheTests`, per-module tests

**Compliance Signals**
- `swift test --filter ModuleHandleContract`, `ModuleColdCache`.
- Review checkpoint: a new module adds handle-contract coverage.

**Current Known Deviations**
- Handle-contract coverage is partial; only Snippets and Browser Tabs have explicit hot-path handle assertions; `WindowsModule` has no dedicated test (`MODULE_MATRIX.md`).

### C-TEST-003: Every AppKit Executor-Boundary Rule Has A Lint/Test Defense

**Contract**
Each rule in `docs/swift6-appkit-boundaries.md` MUST have a lint (`scripts/scan_appkit_executor_risk.sh`) or test (`AppKitExecutor`/`LumaLinterTests`) defense so regressions are caught before merge.

**Rationale**
- `docs/swift6-appkit-boundaries.md` Verification: scanner exits 0, `swift test --filter AppKitExecutor` passes, manual smoke.
- Phase 0: three `.ips` crashes of the class this ADR targets (`CURRENT_STATE.md`).

**Applies To**
- `scripts/scan_appkit_executor_risk.sh`, `Tests` (`AppKitExecutor`, `LumaLinterTests`, `LauncherPanelExecutor`, `HotkeyToggle`)

**Compliance Signals**
- `scripts/scan_appkit_executor_risk.sh` exits 0; `swift test --filter AppKitExecutor` passes.
- `docs/QA.md` Automated Gates run before merge.

**Current Known Deviations**
- `scripts/scan_appkit_executor_risk.sh` has uncommitted edits of unknown origin; its current pass state after those edits is not established (`CURRENT_STATE.md`).

### C-TEST-004: Every Main Path Maps To `PRODUCT_FLOWS.md`

**Contract**
Every end-to-end main path MUST be traceable from a test or QA checklist to a flow in `PRODUCT_FLOWS.md`, and tests that model production behavior MUST NOT silently diverge from the production wiring they claim to represent.

**Rationale**
- `PRODUCT_FLOWS.md` Flow 6: harness models a subset of production wiring.
- P2.5 partial align: production `CommandRegistry`, `configureGlobalSearchModuleIDs`, `applyEnabledSet`; `launcherFlowHarnessReplaysQuery` passes (`P2_EXIT_SUMMARY.md`).
- *(Phase 0/3 historical evidence only: harness had empty `CommandRegistry` and `launcherFlowHarnessReplaysQuery` failed — `CURRENT_STATE.md`.)*

**Applies To**
- `LauncherFlowHarness`, `LauncherGoldenReplayTests`, flow-level tests
- `PRODUCT_FLOWS.md`

**Compliance Signals**
- Review checkpoint: a flow-level test's wiring matches (or documents its divergence from) `AppCoordinator`/`ModuleBootstrapper`.
- `swift test` flow-harness suites pass and map to a named flow.

**Current Known Deviations**
- Full `LauncherFlowHarness` ↔ `AppCoordinator` parity gap remains (P3.2). Partial align P2.5; `launcherFlowHarnessReplaysQuery` passes.

### C-REVIEW-001: Review Judges Against CONTRACTS.md Before Adding Local Rules

**Contract**
Subsequent reviews MUST first determine whether a change conforms to or violates CONTRACTS.md (and whether it adds/removes a Current Known Deviation), rather than accumulating unbounded local ad-hoc rules. New durable rules MUST be reconciled into this file, `docs/ENGINEERING.md`, or `docs/DECISIONS.md`.

**Rationale**
- Phase 0 subjective reports: "之前做的几轮审查，让项目越来越乱" (`CURRENT_STATE.md`) — repeated local patching without a single rubric increased disorder.
- The whole point of Phase 4 is to provide that rubric.

**Applies To**
- All future PRs, reviews, and refactors
- CONTRACTS.md, `docs/ENGINEERING.md`, `docs/DECISIONS.md`

**Compliance Signals**
- PR template/checklist references CONTRACTS.md contract IDs.
- Review artifacts cite contract IDs and deviation status.

**Current Known Deviations**
- No PR checklist currently references CONTRACTS.md (this file is new in Phase 4).

---

## Contract Summary Table

| Contract ID | Short Name | Applies To | MUST / SHOULD | Current Known Deviation? | Compliance Signal |
| --- | --- | --- | --- | --- | --- |
| C-LAYER-001 | App owns UI, not module logic | `LumaApp` | MUST | Yes | Import/review audit; ENGINEERING layer table |
| C-LAYER-002 | Core is pure primitives | `LumaCore` | MUST | Yes | Package deps; no AppKit import |
| C-LAYER-003 | Modules own business, not AppKit | `LumaModules` | MUST | Yes | Import audit; Package deps |
| C-LAYER-004 | Services wrap system APIs | `LumaServices` | MUST | Yes | Grep for direct platform calls |
| C-LAYER-005 | Infra owns logging/metrics/config | `LumaInfrastructure` | MUST | Yes | Dependency/import audit |
| C-LAYER-006 | Acyclic deps per Package.swift | All targets | MUST | Yes (test edge) | `swift build`; Package diff |
| C-HOT-001 | handle memory-only | All modules; QueryDispatcher | MUST | Yes | ModuleHandleContractTests; source scan |
| C-HOT-002 | No unbounded work on hot path | Query dispatch; handle | MUST | Yes | Keystroke perf tests; latency-report |
| C-HOT-003 | Global fan-out small and owned | GlobalSearchTiers | MUST | No | GlobalSearchDispatch tests |
| C-HOT-004 | Bounded targeted first frame | dispatchTargeted | MUST | Yes | ColdTargetedFirstSnapshot tests |
| C-HOT-005 | Timeouts produce diagnostics | QueryDispatcher | MUST | Yes | TimeoutTests |
| C-HOT-006 | Cache excludes sensitive | QuerySnapshotCache | MUST | No | QuerySnapshot; SecretsVault tests |
| C-MODULE-001 | Complete manifest | All modules | MUST | Yes | BuiltInModules tests |
| C-MODULE-002 | Distinct lifecycle methods | All modules | MUST | Yes | Warmup/teardown tests |
| C-MODULE-003 | Heavy warmup / read handle / side-effect perform | All modules | MUST | Yes | QueryDispatcher; Timeout tests |
| C-MODULE-004 | Task owner + cancellation | Module tasks | MUST | Yes | Teardown/warmup-policy tests |
| C-MODULE-005 | Few stable default-on | Warmup defaults | MUST | Yes | BuiltInModules; migration tests |
| C-MODULE-006 | Deferred stays out | Windows; deferred | MUST | Yes | Registration audit |
| C-FAIL-001 | Permission failure visible | Permission surfaces | MUST | Yes (partial coverage) | PermissionBanner tests; QA |
| C-FAIL-002 | Cold cache visible | Warm-cache modules | MUST | Yes | ColdCache tests |
| C-FAIL-003 | Unconfigured shows onboarding | Notes; Projects | MUST | No | Module tests; QA |
| C-FAIL-004 | No false success | ActionExecutor; clients | MUST | Yes (client-side unverified) | ActionFailure tests; QA |
| C-FAIL-005 | Consistent diagnostics | All modules | MUST | Yes | ColdCache; HandleContract tests |
| C-FAIL-006 | Corruption visible to doctor | Persistence; doctor | MUST | Yes | JSONConfigPersistence; Doctor tests |
| C-UI-001 | Single visibility owner | WindowController + Session | MUST | Yes | VisibilitySession tests; QA |
| C-UI-002 | Query text path | SearchBar→VM | MUST | No | QueryView; PermissionBannerContext |
| C-UI-003 | Single content-mode owner | ContentCoordinator | MUST | Yes (doc location) | LauncherSession tests |
| C-UI-004 | Selection bridge | ListView↔Coordinator | MUST | Yes (fallback-to-0) | RowReuse; Snapshot tests |
| C-UI-005 | Detail entry/exit paths | Detail owners | MUST | No | DetailHierarchy; SearchDetailMode |
| C-UI-006 | No owner bypass | All UI-state writers | MUST | Yes (menu Show) | Review; state tests |
| C-APPKIT-001 | Executor-boundary overrides | AppKit subclasses | MUST | Yes (scanner edits) | scan script; AppKitExecutor |
| C-APPKIT-002 | Nonisolated + MainActor bridge | Callbacks/observers | MUST | No | scan script; HotkeyToggle |
| C-APPKIT-003 | Task@MainActor is a bridge | Launcher async | SHOULD | Not audited | Review; CancellationGeneration |
| C-ASYNC-001 | Task owner+cancel+guard | All tasks | MUST | Yes | Cancellation tests |
| C-ASYNC-002 | Generation guards | Show/hide/detail/restore | MUST | Yes (crossfade) | VisibilitySession; ApplyPolicy |
| C-ASYNC-003 | No hidden repaint / hotkey block | Background refresh; show | MUST | Yes (hotkey p95) | BackHome; PanelSignals; latency |
| C-DETAIL-001 | Detail doesn't drive lifecycle | Detail views | MUST | Yes (Wordbook action) | DetailHierarchy |
| C-DETAIL-002 | Detail open via presenter | Detail owners | MUST | No | DetailHierarchy |
| C-DETAIL-003 | Reuse generation contract | Registry; NotesGate | MUST | No | DetailHierarchy; NotesRefresh |
| C-DETAIL-004 | Exit via planner | ExitPlanner | MUST | No | ExitPlanner tests; QA |
| C-DETAIL-005 | No fake open-detail | Registry vs bareBehavior | MUST | Yes (unconfirmed reach) | Cross-check matrix |
| C-CACHE-001 | Cache owner/scope/invalidation | All caches | MUST | Yes (clipboard size) | QuerySnapshot; PanelSignals |
| C-CACHE-002 | Cache hides nothing | Caches; providers | MUST | Yes (corruption) | ColdCache; QuerySnapshot |
| C-PERSIST-001 | Defined corruption/read behavior | Persistence formats | MUST | Yes | JSONConfigPersistence tests |
| C-PERSIST-002 | Doctor reports corruption | Registry; doctor | MUST | Yes | Doctor tests |
| C-DIAG-001 | Export reachable/redacted/local | Commands; Export | MUST | Yes | Redaction tests; reachability |
| C-DIAG-002 | Declared fields populated | buildPayload | MUST | Yes | Redaction tests; doc check |
| C-DIAG-003 | Crash-write failure explainable | CrashLogBuffer | SHOULD | Yes | RedactionAtWrite; doc check |
| C-DIAG-004 | Consistent documented paths | Export/CrashLog; docs | MUST | Yes | Doc check |
| C-DEFAULT-001 | Few stable default-on | Warmup defaults | MUST | Yes (perms doc) | BuiltInModules tests |
| C-DEFAULT-002 | Useful without sensitive perms | Default-on modules | MUST | No | Permission QA |
| C-DEFAULT-003 | Expert/heavy default-off | Warmup defaults | MUST | Yes | BuiltInModules tests |
| C-DEFAULT-004 | Consistent defaults across sources | Docs; manifests | MUST | Yes | Doc check |
| C-DEFAULT-005 | Diagnostics not fully cut off | Commands; Settings | MUST | Yes | Reachability review |
| C-TEST-001 | P0 flow test/QA coverage | 16 flows | MUST | Yes | swift test; QA mapping |
| C-TEST-002 | Module handle coverage | All modules | MUST | Yes | HandleContract; ColdCache |
| C-TEST-003 | Executor-boundary defense | scan; AppKitExecutor | MUST | Yes | scan script; tests |
| C-TEST-004 | Tests map to flows | Flow harnesses | MUST | Yes | Review; harness suites |
| C-REVIEW-001 | Review against contracts | All PRs | MUST | Yes | PR checklist references IDs |

---

## Current Known Deviations

Consolidated list of currently-known facts that do not satisfy a contract. Facts only; no fix prescribed here.

1. ~~**`docs/PERMISSIONS.md` Default column is stale**~~ — **Resolved P2.1 (2026-07-07):** Default column matches manifests + D-012; "Menu Bar Search" naming aligned. (C-DEFAULT-004)
2. **Windows `handle` calls `CGWindowListCopyWindowInfo` directly** while deferred/not registered. Manifest `defaultEnabled: false` since P2.1; module remains in `BuiltInModules.makeDeferred()` only. (C-MODULE-006, C-HOT-001, C-LAYER-004) — `MODULE_MATRIX.md`.
3. ~~**Commands default-off makes `cmd doctor` / `cmd export-diagnostics` unreachable on a fresh install**~~ — **Resolved P0.8/P3.1 (2026-07-07):** menu bar **Run Doctor…** / **Export Diagnostics…** via `AppHostService`; `LUMA_QA_EXPORT` smoke validates `diagnostics.json`. (C-DEFAULT-005, C-DIAG-001)
4. ~~**Diagnostics payload `platform`/`modules`/`permissions`/`recentErrors` are empty/`null`**~~ — **Resolved P0.8/P2.5:** `RecoveryDiagnosticsCollector.buildExportPayload` populates all sections; P2 Exit smoke gate. (C-DIAG-002)
5. ~~**`crash-log.txt` path mismatch**~~ — **Resolved P3.1 (2026-07-07):** documented at `~/Library/Application Support/Luma/crash-log.txt`; export includes `crashLogPath`/`crashLogWriteStatus`. (C-DIAG-004). **Partial:** `CrashLogBuffer` write failure has no in-app alert unless export runs (C-DIAG-003).
6. **`ConfigCorruptionRegistry` is purely in-memory** — does not survive restart; corruption from a prior run is not shown after relaunch. (C-PERSIST-002, C-FAIL-006) — `PRODUCT_FLOWS.md` Flow 15.
7. **`JSONConfigPersistence` read failures (I/O/permission) are silent fallback, not quarantined/registered** — only decode failures reach the corruption registry. (C-PERSIST-001, C-FAIL-006) — `PRODUCT_FLOWS.md` Flow 15.
8. **`JSONFileStore` (Snippets/Media) and `ClipboardHistoryStore` use separate quarantine paths**; `JSONFileStore` does not feed `ConfigCorruptionRegistry`, so their corruption is not confirmed in `cmd doctor`. (C-PERSIST-002, C-FAIL-006) — `PRODUCT_FLOWS.md` Flow 15, `MODULE_MATRIX.md`.
9. **`handle` memory-only has no generic enforcement** — only per-module proxy tests; no type-system or static-analysis guard. (C-HOT-001) — `MODULE_MATRIX.md`.
10. **Diagnostic behavior is inconsistent across modules** — non-`.queryable` targeted modules return silent empty vs disabled modules' diagnostic row; bespoke cold strings coexist with the generic `module.warming` row; "empty acceptable" vs "diagnostic required" is decided per-module. (C-FAIL-005, C-HOT-005) — `PRODUCT_FLOWS.md` Flows 6/7, `MODULE_MATRIX.md`.
11. **`HotkeyConfig.save()` is a no-op** (Command+Space fixed by design); if any Settings UI implies a configurable chord, it would silently no-op on save (Settings UI not confirmed). (C-UI-002 adjacent; product/UX consistency) — `PRODUCT_FLOWS.md` Flow 2.
12. **Menu bar "Show" bypasses the Carbon show guard/debounce** by calling `show()` directly, undocumented in the two-paths description. (C-UI-001, C-UI-006) — `PRODUCT_FLOWS.md` Flow 3.
13. **`LauncherFlowHarness` full production parity gap** — P2.5 partial align (`CommandRegistry`, `globalSearchModuleIDs`, `applyEnabledSet`; `launcherFlowHarnessReplaysQuery` passes). Full `AppCoordinator` E2E parity remains P3.2. (C-TEST-004) — `P2_EXIT_SUMMARY.md`.
14. **Hotkey p95 ≈ 8.3 s** — ~100× the documented 50 ms p95 / 80 ms ceiling for hotkey→interactive/home. (C-HOT-002, C-ASYNC-003) — `CURRENT_STATE.md`.
15. **Three `.ips` crashes (SIGSEGV/SIGABRT)** on 2026-07-06 in the AppKit executor-boundary risk class; the app process was not running at Phase 0 snapshot. (C-APPKIT-001, C-TEST-003) — `CURRENT_STATE.md`.
16. **`scripts/scan_appkit_executor_risk.sh` and `LauncherListView.swift` have uncommitted edits of unknown origin** — the scanner's current pass state after its own edits is not established. (C-APPKIT-001, C-TEST-003) — `CURRENT_STATE.md`.
17. ~~**Layer/ownership doc mismatches** (`LauncherContentMode` location; diagnostics types attributed only to `LumaInfrastructure`)~~ — **Resolved P3.1 (2026-07-07)** in `docs/ENGINEERING.md` / `docs/PERMISSIONS.md`. **Remaining facts:** `LauncherSessionState` is test-only (not production SoT); `AppCoordinator` instantiates `ProjectsModule` for path matching; Workbench capture split across app/module layers. (C-LAYER-001/002/005, C-UI-003)
18. **Full `PermissionResultBuilder.row` call-site enumeration is unconfirmed** beyond Todo and Window Layouts — permission-visibility coverage across all permissioned modules is not fully verified. (C-FAIL-001) — `ARCHITECTURE_MAP.md`, `MODULE_MATRIX.md`.
19. **Platform clients' throw-on-no-op is unverified** — the `ActionExecutor` propagation contract holds, but each `PasteboardClient`/`AccessibilityClient`/`WorkspaceClient` path was not confirmed to throw rather than silently no-op. (C-FAIL-004) — `PRODUCT_FLOWS.md` Flow 9.
20. **Startup/hotkey-failure/idle-teardown lack located end-to-end tests**; `KillProcessModule` has no explicit teardown; show-path menu-tree Task cancellation-on-hide unconfirmed. (C-TEST-001, C-MODULE-004, C-ASYNC-001) — `PRODUCT_FLOWS.md` Flows 1/2/3/14.
21. **`LauncherSessionState` is test-only** — reducer + four legacy effect hooks; not production source of truth for panel/query/selection (runtime owners: `LauncherRootController`, `LauncherContentCoordinator`, `LauncherPanelVisibilitySession`). Delete vs promote decision deferred. (`docs/ENGINEERING.md`, `LAUNCHER_STATE_AUDIT.md`)

---

## Contract-To-Phase Mapping

| Contract | Phase Facts |
| --- | --- |
| C-LAYER-001..006 | P1: Package target graph, layer ownership table, `AppCoordinator` no-DI, diagnostics/`LauncherContentMode` split-location facts; P2: Workbench split |
| C-HOT-001 | P1: handle memory-only documented not enforced; P2: Windows deferred `handle` CGWindow violation; P3: global/targeted hot-path flows |
| C-HOT-002 | P0: hotkey p95 ≈ 8.3s; P1/P2: no-unbounded-work docs; P3: Flows 5/6/7 dispatch + timeouts |
| C-HOT-003 | P2: contributing set apps/quicklinks/clipboard; P3: Flow 6 fan-out tiers |
| C-HOT-004/005 | P3: Flow 7 warming-row-before-warmup; Flow 6/7 timeout → diagnostic row |
| C-HOT-006 | P1/P2: secrets/snippets excluded from QuerySnapshotCache; D-* decisions |
| C-MODULE-001..006 | P2: manifests/warmup tiers/default states; Windows deferred; P3: Flow 14 lifecycle |
| C-FAIL-001 | P2/P3: permission rows (Todo/WindowLayouts/MenuItems/BrowserTabs/Snippets/Secrets); Flow 13 |
| C-FAIL-002 | P2: cold-cache rows; P3: Flow 14 warmup timeout silent-cold |
| C-FAIL-003 | P2: Notes/Projects onboarding rows; P3 Flows 4/7 |
| C-FAIL-004 | P3: Flow 9 perform-then-dismiss; client throw-on-no-op unverified |
| C-FAIL-005 | P2: diagnostic asymmetry; P3: Flow 7 non-queryable silent empty vs disabled diagnostic |
| C-FAIL-006 | P3: Flow 15 registry-blind JSONFileStore, read-failure fallback, in-memory registry |
| C-UI-001..006 | P1/P3: Cross-Cutting State Owners; Flow 3 menu-Show bypass; Flow 8 selection bridge |
| C-APPKIT-001..003 | P0: 3 `.ips` crashes; swift6-appkit-boundaries ADR; P0 uncommitted scanner edits |
| C-ASYNC-001..003 | P0: hotkey p95; P3: Flows 3/6/11/12/14 generation guards, cancellation gaps |
| C-DETAIL-001..005 | P2: detail registry; P3: Flows 10/11 presenter/exit-planner, no-detail modules |
| C-CACHE-001/002 | P1/P2: cache TTLs, clipboard size; P3: Flow 15 corruption hidden by fallback |
| C-PERSIST-001/002 | P3: Flow 15 three persistence paths, read vs decode failure, in-memory registry |
| C-DIAG-001..004 | P0/P2: export reachable + populated payload; P3.1: paths documented |
| C-DEFAULT-001..005 | P2.1: defaults aligned; P0.8/P3.1: recovery entry documented |
| C-TEST-001..004 | P0: failing harness test, missing tests; P3: harness/production divergence; QA gates |
| C-REVIEW-001 | P0: "越来越乱" subjective report; Phase 4 rubric purpose |

---

## Non-Goals

This phase (Phase 4) intentionally does **not**:

- Write concrete refactor steps or a "change file X" plan.
- Specify PR ordering or a migration sequence.
- Modify any Swift source code.
- Modify or add any test.
- Rule on which modules are ultimately deleted, merged, or kept.
- Diagnose root causes (hotkey p95 ≈ 8.3 s, the three `.ips` crashes, `launcherFlowHarnessReplaysQuery`).
- Treat any Current Known Deviation as a target. Deviations are recorded facts to be resolved to the contract or explicitly re-decided in `docs/DECISIONS.md`, never fossilized as the goal.
- Run `swift build`, `swift test`, or any script (all facts here come from the Phase 0-3 artifacts and the engineering docs).
