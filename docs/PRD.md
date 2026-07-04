# Luma PRD

## Summary

Luma is a personal macOS launcher for one keyboard-heavy developer. It opens via Command+Space, provides Raycast-like command speed, Spotlight-like visual calm, and a local-first command surface for the user's highest-frequency actions.

## Strategic Update

The active product strategy is **Route C** (ADR-023): a command-first unified launcher. Command+Space opens the ADR-032 home: **Open Apps** in the left column and a compact module guide/detail area on the right. Non-empty queries use flat search results. Module detail views remain in the same panel; dashboard feature cards and home suggestion sections are not used.

Authoritative planning docs:

- `docs/adr/023-command-first-unified-list.md`
- `docs/PRD.md` (this file)
- `docs/ARCHITECTURE.md`
- `docs/OPUS_DECISIONS.md`

Historical route docs:

- `docs/adr/007-dashboard-widget-single-window.md` (Route B, superseded)
- `docs/adr/006-launcher-convergence.md` (Route A, historical)

## Product Goals

- Native AppKit launcher with p95 hotkey-to-interactive <= 50 ms.
- Empty-query home: **Open Apps** in the left column + command guide or module detail on the right (ADR-032; calm app switcher with discoverability).
- Modular query/action implementation behind a simple launcher UX.
- Local-first data for clipboard, usage, configuration, and app cache.
- A small core feature set that remains fast under repeated daily use.

## Current Phase

The product is already mostly formed. Current work should prioritize:

- wiring existing flows together
- improving permissions and recovery paths
- smoothing keyboard-only behavior
- tightening visual consistency and empty states
- fixing trust gaps before adding scope

## Required Features

1. App Search / Launcher.
2. Clipboard History.
3. Translate typed or selected text.
4. Open Apps section ordered by activation recency/frequency.
5. Command-first module triggers (`clip`, `note`, `t`, `tr`, etc.).
6. **Window Layout** — prefix-triggered presets (`layout` / `win` / `wl`) to move the focused window (left/right/top/bottom/maximize/center).
7. **Project Switcher** — prefix-triggered project open (`proj` / `p` / `project`) with config at `~/Library/Application Support/Luma/projects.json`.
8. Spotlight/Raycast-like UI: **940 × 760 pt** panel, upper-third placement, empty home uses Open Apps (left) + command guide (right).
9. `Features/` folder containing per-module introductions and maintenance notes.

**Frozen home constraints:** `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md` + ADR-032 — Open Apps left column; no setup/recent/continue/create rows without ADR.

**Frozen panel constraints:** `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md` — 940×760 geometry, presentation-screen placement, no full-width `wantsLayer`.

**Frozen Notes detail constraints:** `docs/specs/NOTES_DETAIL_CONSTRAINTS.md` — chip/panel IA, create flows, Tree/Map toggle, in-detail shortcuts.

**Open navigation/shortcut gaps (temporary):** `docs/qa/LAUNCHER_NAVIGATION_AUDIT.md` — detail exit, keyboard routing, session restore; align `docs/specs/UX_BEHAVIOR_RULES.md` when closed.

## Active Built-In Modules

Registered at launch (`BuiltInModules.makeAll()`): Apps, Clipboard, Commands, Notes, Todo, Translate, Wordbook, Snippets, Secrets, Records (`luma.media`), Window Layouts, Projects, Quicklinks, Menu Bar Search, Kill Process, Browser Tabs, Auto Workflow.

Default off in Settings: Commands, Records (`luma.media`), Browser Tabs, Auto Workflow.

Deferred (no warmup, not in default registry): Windows (window focus list).

## Deferred Or Experimental Scope

- **Windows module** — full window picker/focus (deferred; distinct from Window Layouts presets).
- Notes Graph / wiki-link graph views (folder-tree Mind Map in detail is in scope per ADR-012/017).
- Plugin marketplace or public extension API.
- Thirds/quarters/saved multi-window split layouts (Window Layouts v2).

## Wordbook Requirements

Wordbook is an active built-in module (`word` trigger) with same-panel review (ADR-013). Keep `/Users/diaoyuxuan/wordbot` as migration source of truth until a later dedicated phase.

## Success Criteria

- Hotkey -> visible interactive panel: p95 <= 50 ms.
- Keystroke -> first ranked snapshot painted: p95 <= 30 ms.
- Adding or maintaining a feature starts from its module folder, ADR, or `Features/` note when one exists.
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
