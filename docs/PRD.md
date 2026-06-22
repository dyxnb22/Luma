# Luma PRD

## Summary

Luma is a personal macOS launcher for one keyboard-heavy developer. It opens via Command+Space, provides Raycast-like command speed, Spotlight-like visual calm, and a local-first command surface for the user's highest-frequency actions.

## Strategic Update

The active product strategy is Route B: a single Dashboard Widget window that keeps Command+Space as the primary entry point while combining search, open-app context, widget cards, and same-panel module details.

Authoritative planning docs:

- `docs/strategy/PRODUCT_ROUTE_OPTIONS.md`
- `docs/adr/007-dashboard-widget-single-window.md`
- `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
- `docs/strategy/DASHBOARD_WIDGET_POLISH_PLAN.md`

Historical Route A reference:

- `docs/adr/006-launcher-convergence.md`
- `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md`
- `docs/strategy/CONVERGENCE_EXECUTION_PLAN.md`

## Product Goals

- Native AppKit dashboard launcher with p95 hotkey-to-interactive <= 50 ms.
- Empty-query dashboard with open apps, widget cards, and lightweight module detail views.
- Modular query/action implementation behind a simple launcher UX.
- Local-first data for clipboard, usage, configuration, and app cache.
- A small core feature set that remains fast under repeated daily use.

## Required Features

1. App Search / Launcher.
2. Window Focus.
3. Clipboard History.
4. Translate typed or selected text.
5. Open Apps sidebar ordered by actual activation recency/frequency.
6. Quick Calculator.
7. Spotlight/Raycast-like UI with native macOS liquid-glass visual calm.
8. `Features/` folder containing per-module introductions and maintenance notes.

## Deferred Or Experimental Scope

- Notes Graph.
- Wordbook.
- Secrets Vault.
- Window Layout engine.
- Plugin marketplace or public extension API.

## Wordbook Requirements

Wordbook is not part of the active six-module dashboard core. Keep `/Users/diaoyuxuan/wordbot` as the source of truth until a later dedicated migration phase.

## Success Criteria

- Hotkey -> visible interactive panel: p95 <= 50 ms.
- Keystroke -> first ranked snapshot painted: p95 <= 30 ms.
- Adding or maintaining a feature starts from its folder in `Features/`.
- Every user-facing feature has a `LumaModule` boundary.
- The dashboard shell stays lightweight: no warmup for deferred modules.
- Module detail pages stay same-panel and do not become separate app windows.

## Platform

- macOS 14+.
- Swift 6 strict concurrency.
- AppKit for launcher.
- SwiftUI allowed for Settings/About only.
- Developer ID signed and notarized DMG for v1.
