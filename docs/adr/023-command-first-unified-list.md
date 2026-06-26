# ADR-023: Command-First Unified List (Route C)

## Status

Accepted. Supersedes ADR-007 for UI implementation direction.

Date: 2026-06-25

## Context

ADR-007 (Route B) established a dashboard widget layout: fixed sidebar, feature card grid on empty query, and search results in a separate region. That layout optimizes for module discovery at the cost of an extra navigation step and split keyboard semantics (⌘N jumps cards on home, results when searching).

Luma's differentiation is low-latency command execution, not module count. The dashboard turns the launcher into a module picker.

ADR-006 (Route A) converged on a pure launcher but excluded same-panel module detail. Route C keeps detail-in-panel while removing dashboard cards and the permanent sidebar.

## Decision

Route C — **Command-First Unified List**:

1. **Command-first**: First-screen elements must be reachable in one keyboard step.
2. **One list, one column**: A single vertical list with sections (home) or a flat list (search results).
3. **No dashboard**: Feature cards are not home-screen entry points. Module detail views may retain card-style headers.
4. **Module = result provider**: Modules surface via `ResultItem` rows, not `FeatureCard` tiles.
5. **Open Apps as section #1**: Open apps appear in the home list, not a permanent sidebar.
6. **Action Panel is opt-in**: Tab / ⌘K opens an action chooser; rows show primary action on Return.
7. **Minimal home abstraction**: Two home providers only — Open Apps, Suggested.
8. **Performance budget**: Each home provider ≤ 4 ms on main thread; full home snapshot ≤ 16 ms.

Panel geometry: default **900 × 600 pt**, responsive **840–940 × 580–700 pt** (58% screen width × 66% height, capped). Larger than historical Route B's 860 × 540 widget shell.

## Consequences

Positive:

- Unified ⌘1–9 semantics across home and results.
- Less visual chrome; more list density for commands.
- Aligns hot path with QueryDispatcher (unchanged).

Negative:

- No visible multi-module grid; users rely on triggers and suggestions.
- Home may show the full ranked Open Apps section without a collapsed "+N more" step.
- ADR-007 dashboard strategy docs become historical reference.

## Implementation Notes

- `LauncherListView` replaces `FeatureFlowView` + sidebar + dual scroll views.
- `LauncherHomeAggregator` composes two providers on empty query only.
- `FeatureCatalog.dashboardCoreCards()` may remain for detail header metadata until a later cleanup.
