# Luma PRD

## Summary

Luma is a personal macOS launcher for one keyboard-heavy developer. It opens via Command+Space, provides Raycast-like command speed, Spotlight-like visual calm, and a local-first command surface for the user's highest-frequency actions.

## Strategic Update

The active product strategy is launcher convergence: make Command+Space -> type -> result -> action excellent before expanding the dashboard/workbench scope.

Authoritative planning docs:

- `docs/strategy/PRODUCT_ROUTE_OPTIONS.md`
- `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`
- `docs/strategy/CONVERGENCE_EXECUTION_PLAN.md`

Alternative dashboard/widget implementation docs:

- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
- `docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md`

## Product Goals

- Native AppKit launcher with p95 hotkey-to-interactive <= 50 ms.
- Usage-based empty-query recents and frequent actions.
- Modular query/action implementation behind a simple launcher UX.
- Local-first data for clipboard, usage, configuration, and app cache.
- A small core feature set that remains fast under repeated daily use.

## Required Features

1. App Search / Launcher.
2. Window Focus.
3. Clipboard History.
4. Translate typed or selected text.
5. Frecency Recent Items.
6. Quick Calculator.
7. Spotlight/Raycast-like UI with native macOS visual calm.
8. `Features/` folder containing per-module introductions and maintenance notes.

## Deferred Or Experimental Scope

- Dashboard cards.
- Notes Graph.
- Wordbook.
- Secrets Vault.
- Window Layout engine.
- Plugin marketplace or public extension API.

## Wordbook Requirements

Wordbook is no longer part of the v1 launcher path. Keep `/Users/diaoyuxuan/wordbot` as the source of truth or split it into a separate app later.

## Success Criteria

- Hotkey -> visible interactive panel: p95 <= 50 ms.
- Keystroke -> first ranked snapshot painted: p95 <= 30 ms.
- Adding or maintaining a feature starts from its folder in `Features/`.
- Every user-facing feature has a `LumaModule` boundary.
- Empty query results are driven by real usage/frecency data.
- The launcher panel does not become a dashboard or detail page.

## Platform

- macOS 14+.
- Swift 6 strict concurrency.
- AppKit for launcher.
- SwiftUI allowed for Settings/About only.
- Developer ID signed and notarized DMG for v1.
