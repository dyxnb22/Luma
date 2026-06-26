# Module Contract Spec

## Protocol

Modules implement `LumaModule`. Concrete modules should be actors unless they are stateless.

Required semantics:

- `manifest`: static, nonisolated metadata.
- `warmup`: load indexes and caches; 1 second soft budget.
- `handle`: answer a query from memory only; hard timeout <= 80 ms.
- `perform`: execute custom actions; 2 second soft budget.
- `teardown`: cancel background work and flush state.

## Rules

Modules must:

- Produce stable `ResultID` values.
- Respect cancellation during loops.
- Use `ModuleContext` for shared services.
- Keep disk and network I/O out of `handle`.
- Return diagnostics instead of throwing from `handle`.
- Keep state private to the module actor.

Modules must not:

- Touch AppKit views.
- Reach into other modules.
- Spawn detached tasks to bypass host timeouts.
- Use `DispatchQueue` for new async work.
- Persist outside their own schema namespace.

## Timeout Policy

The host enforces timeouts. A timeout becomes an empty module result plus a diagnostic. Missing Accessibility permission becomes `ModuleDiagnostic.Kind.permissionRequired`. One slow or permission-blocked module must never delay the whole launcher.

## Action Routing

Host-handled action kinds:

- Launch app.
- Focus window.
- Copy to pasteboard.
- Open URL.
- Reveal in Finder.
- Insert text.

Custom actions route back to the originating module.

Launcher-handled UI intents (not executed by `ActionExecutor`):

- `openModuleDetail` — present module detail inside the launcher panel.
- `replaceQuery` — replace the search field text (help rows, unknown-prefix suggestions).
- `translateText` — open Translate detail with prefilled text.

## Interaction types

| Type | Return behavior | Panel dismisses? |
|------|-----------------|------------------|
| immediate | Execute side effect | Yes |
| in-panel | Open or advance detail | No |
| capture | Create data from query | Yes |
| search/browse | Follow row action type | Depends on row |
| navigate-only | Replace query or no-op | No |

Modules declare intent via `Action.kind` and `ResultItem.rowKind`; the launcher interprets them.

## Wordbook v0.2 interfaces

- `WordFamiliarity`: `.known` (stage+1), `.unknown` (reset), `.mastered` (permanent, no longer due), `.fuzzy` (legacy → schedules as `.known`).
- `WordbookProgressSnapshot`: single-query home card stats (`total`, `mastered`, `dueToday`, `dailyNewLimit`, `streakDays`, etc.).
- `WordbookSessionPlanner` actor: `startNewSession()`, `nextCard() -> .review | .fresh | .done`.
- `WordbookStore`: `progressSnapshot()`, `dailyNewLimitForDueCount`, `resetDailyStatsIfNeeded`, `upsertWords`.
- Detail entry follows the Route C dispatch path: row `Action(kind: .openModuleDetail(.wordbook, payload:))`. `LauncherRootController` reads `WordbookAction.review` from the payload and sets `LauncherSharedState.pendingWordbookAutoStartReview` before `WordbookDetailView.activate()` consumes the flag. Modules do not call `LauncherCallbackRegistry.openModuleDetail` directly.
