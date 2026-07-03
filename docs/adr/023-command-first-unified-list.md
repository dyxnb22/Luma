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
7. **Minimal home abstraction**: **Open Apps only** on empty query. Suggested/continue/create rows are **not** rendered on home (frozen 2026-07-03; see `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`).
8. **Performance budget**: Open Apps home provider ≤ 4 ms on main thread; full home snapshot ≤ 16 ms.

Panel geometry (2026-07-03): default **720 × 680 pt**, responsive **640–760 × 600–760 pt**, upper-third bias (`panelVerticalBias` 0.68). Supersedes the earlier 900 × 600 wide-shell note below.

Historical panel note (pre-2026-07): ~~900 × 600 pt, 58% screen width × 66% height~~.

In-panel layout (2026-07-03): full-width hosts must not use `wantsLayer` (default layer `anchorPoint` causes horizontal drift when hints/results/detail relayout). See `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`.

## Consequences

Positive:

- Unified ⌘1–9 semantics across home and results.
- Less visual chrome; more list density for commands.
- Aligns hot path with QueryDispatcher (unchanged).

Negative:

- No visible multi-module grid; users rely on triggers and search.
- No home suggestion rows; workbench continue/create flows use commands and detail instead.
- Full Open Apps list (no `+N more` collapse).
- ADR-007 dashboard strategy docs become historical reference.

## Implementation Notes

- `LauncherListView` replaces `FeatureFlowView` + sidebar + dual scroll views.
- `LauncherHomeAggregator` composes **Open Apps only** on empty query (frozen).
- `FeatureCatalog.moduleDetailMetadata()` may remain for detail header chrome until fully inlined.
- Authoritative freeze: `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`.
- In-panel layout freeze: `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`.
