# ADR-006: Launcher Convergence

## Status

Accepted as active strategic direction for v1 planning.

## Context

Luma accumulated multiple competing product identities:

- Spotlight replacement.
- Raycast-like command launcher.
- Dashboard/workbench with draggable cards.
- Notes graph.
- Wordbook.
- Secrets vault.
- Window manager.

The product risk is not lack of features. The risk is product drift: too many adjacent tools competing with the launcher hot path.

## Decision

Luma v1 will converge on a pure launcher model:

> Command+Space -> query -> ranked results -> action.

The launcher panel should not be a dashboard and should not contain module detail pages. Empty query shows usage-backed recents/frequents. Non-empty query shows ranked results.

The v1 core feature set is:

1. App Search / Launcher
2. Window Focus
3. Clipboard History
4. Translate
5. Frecency Recent Items
6. Quick Calculator

Dashboard Cards, Notes Graph, Wordbook, Secrets Vault, and Window Layout engine are deferred or experimental and should not shape the default launcher UX.

## Consequences

Positive:

- Lower maintenance surface.
- Cleaner first-run UX.
- Stronger latency discipline.
- Fewer modules competing for the query hot path.
- A clearer answer to what Luma is.

Negative:

- Some already-built modules may be hidden, disabled, or removed from the default path.
- The project gives up the broader personal-workbench ambition for now.
- Existing docs and tests must be periodically checked for stale dashboard assumptions.

## Implementation Notes

- Do not delete experimental modules until the active launcher path is stable.
- Disable or hide non-core modules before deleting them.
- Keep all future features behind a strict question: is this one of the core launcher functions?
- Prefer `ScriptedCommandsModule` over a public plugin runtime.

