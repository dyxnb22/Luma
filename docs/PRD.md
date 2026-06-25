# Luma PRD

## Summary

Luma is a personal macOS launcher for one keyboard-heavy developer. It opens via Command+Space, provides Raycast-like command speed, Spotlight-like visual calm, and a local-first command surface for the user's highest-frequency actions.

## Strategic Update

The active product strategy is **Route C** (ADR-023): a command-first unified list launcher. Command+Space opens a single-column list with sectioned home rows (Open Apps, Suggested, Recent) and flat search results. Module detail views remain in the same panel; dashboard feature cards are no longer home-screen entry points.

Authoritative planning docs:

- `docs/adr/023-command-first-unified-list.md`
- `docs/PRD.md` (this file)
- `docs/ARCHITECTURE.md`

Historical route docs:

- `docs/adr/007-dashboard-widget-single-window.md` (Route B, superseded)
- `docs/adr/006-launcher-convergence.md` (Route A, historical)
- `docs/strategy/PRODUCT_ROUTE_OPTIONS.md`

## Product Goals

- Native AppKit launcher with p95 hotkey-to-interactive <= 50 ms.
- Empty-query home list with Open Apps, contextual suggestions, and recent actions.
- Modular query/action implementation behind a simple launcher UX.
- Local-first data for clipboard, usage, configuration, and app cache.
- A small core feature set that remains fast under repeated daily use.

## Required Features

1. App Search / Launcher.
2. Clipboard History.
3. Translate typed or selected text.
4. Open Apps section ordered by activation recency/frequency.
5. Command-first module triggers (`clip`, `note`, `t`, `tr`, etc.).
6. **Window Layout** — prefix-triggered presets (`layout` / `win` / `wl`) to move the focused window (left/right/top/bottom/maximize/center).
7. **Project Switcher** — prefix-triggered project open (`proj` / `p` / `project`) with config at `~/Library/Application Support/Luma/projects.json`.
8. Spotlight/Raycast-like UI with native macOS visual calm.
9. `Features/` folder containing per-module introductions and maintenance notes.

## Active Built-In Modules

Registered at launch (`BuiltInModules.makeAll()`): Apps, Clipboard, Commands, Notes, Todo, Translate, Wordbook, Snippets, Secrets, Media, Window Layouts, Projects.

Deferred (no warmup, not in default registry): Windows (window focus list).

## Deferred Or Experimental Scope

- **Windows module** — full window picker/focus (deferred; distinct from Window Layouts presets).
- Notes Graph / wiki-link graph views.
- Plugin marketplace or public extension API.
- Thirds/quarters/saved multi-window split layouts (Window Layouts v2).

## Wordbook Requirements

Wordbook is an active built-in module (`word` trigger) with same-panel review (ADR-013). Keep `/Users/diaoyuxuan/wordbot` as migration source of truth until a later dedicated phase.

## Success Criteria

- Hotkey -> visible interactive panel: p95 <= 50 ms.
- Keystroke -> first ranked snapshot painted: p95 <= 30 ms.
- Adding or maintaining a feature starts from its folder in `Features/`.
- Every user-facing feature has a `LumaModule` boundary.
- The launcher shell stays lightweight: no warmup for deferred modules (Windows).
- Module `handle` paths stay memory-only; disk scan belongs in warmup (Projects, Apps, Notes).
- Module detail pages stay same-panel and do not become separate app windows.

## Platform

- macOS 14+.
- Swift 6 strict concurrency.
- AppKit for launcher.
- SwiftUI allowed for Settings/About only.
- Developer ID signed and notarized DMG for v1.
