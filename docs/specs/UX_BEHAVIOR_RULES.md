# UX Behavior Rules

Authoritative home-screen freeze: [LAUNCHER_HOME_CONSTRAINTS.md](LAUNCHER_HOME_CONSTRAINTS.md).  
Panel positioning and in-panel layout: [LAUNCHER_PANEL_CONSTRAINTS.md](LAUNCHER_PANEL_CONSTRAINTS.md).  
Notes detail IA and create flows: [NOTES_DETAIL_CONSTRAINTS.md](NOTES_DETAIL_CONSTRAINTS.md).  
**Open interaction gaps (temporary):** [LAUNCHER_NAVIGATION_AUDIT.md](../qa/LAUNCHER_NAVIGATION_AUDIT.md).

## Launcher

- Hotkey toggles the launcher.
- Default hotkey is Command+Space.
- The launcher panel is not a dashboard.
- Module detail surfaces open in the **right column** when the visible query is empty; Open Apps stay on the left (ADR-032).
- Escape dismisses, or steps back through detail → results (restores query) → home → close panel.
- Tab opens the action panel for secondary actions; Tab or Shift+Tab closes it when open.
- Standard edit shortcuts work in text fields and detail views: Command+A/C/V/X/Z, Command+Shift+Z / Command+Y.
- Global search requires at least 2 characters unless a command prefix is used.
- Module help: `help <trigger>` (IME-friendly) or `<trigger> ?` (e.g. `help clip`, `clip ?`).
- In module detail, the search field shows a read-only placeholder (`In <Module> — Esc to go back`) instead of the prior query.
- **Every** detail exit path (Esc, detail back/close chrome, empty-query home, panel hide) must restore search-field editability — `beginDetailMode` sets `isEditable = false`; callers must pair with `endDetailMode`, `cancelDetailMode`, or `reEnableSearchFieldIfNeeded()` (regression: back from Notes left the home search box unclickable).
- Clicking the list area focuses the list; typing forwards to the search field (including IME `insertText`).

### Navigation state machine (frozen intent)

| Layer | State | Visible UI |
| --- | --- | --- |
| Home | Empty query, `!showingResults`, `!showingDetail` | Left: Open Apps · Right: command guide |
| Results | Non-empty query, `showingResults` | Single-column search results (≤8 rows) |
| Detail | `showingDetail`, empty visible query | Left: Open Apps · Right: module detail (ADR-032) |

**Enter detail:** `enterDetailContext` → `beginDetailMode` → `contentCoordinator.present`.  
**Leave detail (canonical):** `exitDetailFromChrome` — used by Esc, detail **back**, and detail **close** (same behavior).  
**Partial teardown:** `closeDetail` only removes detail UI; must not be the sole user-facing exit (use `exitDetailFromChrome` for chrome).  
**Panel hide:** does not reset detail state; session save uses `searchBar.stringValue` (empty while in detail mode).

### Detail exit matrix

| Path | Restores suspended query? | Restores `isEditable`? | Navigates to |
| --- | --- | --- | --- |
| Esc (in detail) | Yes (`endDetailMode`) | Yes | Results or home |
| Detail back / close | Yes | Yes | Results or home |
| `showHome` | No (`cancelDetailMode`) | Yes | Home |
| `closeDetail` alone | No | Yes (`reEnableSearchFieldIfNeeded`) | Stays on prior list |
| Panel hide | No (memory lost) | N/A | Panel hidden |

### Keyboard shortcuts (intended contract)

| Context | ↑↓ | ⌘1–9 | ⌘↩ | Tab / ⌘K | Esc |
| --- | --- | --- | --- | --- | --- |
| Home / results (search or list focused) | Move selection | Jump + run row | First secondary action (search focused) | Open action panel | Back / clear / close |
| Detail | **Must not** route ↑↓ / ⌘1–9 to the list while detail subviews hold focus (audit K6); left Open Apps stay visible in split — use mouse to select apps or click list then ↑↓ | Module shortcuts via `dispatchDetailKeyDown` | N/A in detail | Swallowed | `exitDetailFromChrome` |
| Action panel open | Move panel selection when panel has key focus | Activate panel row | — | Dismiss panel | Dismiss panel |

**Known gaps:** module-level shortcuts in `handleKeyDown` may be unreachable when detail subviews hold focus — see [LAUNCHER_NAVIGATION_AUDIT.md](../qa/LAUNCHER_NAVIGATION_AUDIT.md). ⌘W close detail is implemented on `LauncherPanel` (audit K1).

- **No auto-present onboarding wizard on first launch.** Home opens directly to Open Apps.
- UI language: Settings → General → Language (English / 简体中文 / System).
- Return runs the selected row's primary action.
- Command+Return runs the first secondary action when present.
- Arrow up/down moves selection.
- Command+number selects or runs the visible nth result (unified across home and results).
- Panel dismisses immediately after immediate (leave-launcher) actions.
- In-panel actions (open detail, replace query, translate) keep the panel visible.
- Results update progressively as modules return.
- Empty query shows **Open Apps** in the left column plus command guide or module detail on the right (ADR-032; see frozen constraints).
- If the raw query exactly matches a snippet trigger word (case-insensitive) in global search mode, Return expands and pastes the snippet inline — the panel dismisses without opening Snippets detail.

## Panel

- AppKit `NSPanel`, borderless, floating, pre-instantiated.
- Shows across Spaces and fullscreen apps.
- Default size **940 × 760 pt**; positioned in the **upper third** of the presentation screen (`panelVerticalBias` 0.68).
- Presentation screen: cursor display → key window display → main (`LumaPresentationScreen.current()`).
- Placement uses **one atomic** `setFrame` via `LauncherPanel.position(on:)`; panel `minSize`/`maxSize` locked after position.
- Responsive clamp: width **720–980** pt, height **640–820** pt when the display is smaller.
- **No** `anchorPoint` or scale transforms on the root content view (causes horizontal clip).
- Show/hide: alpha fade only (`MotionTokens`); no scale animation.
- The query field receives focus on every show.
- Module detail and search results must not widen the panel — scroll or truncate inside fixed width.

### In-panel layout (frozen)

- Full-width hosts (`LauncherRootView`, `contentContainer`, `detailContainer`, list row body, `BaseDetailContainer` root) must **not** use `wantsLayer`. Default layer `anchorPoint (0.5, 0.5)` causes horizontal drift when command hints, sectioned results, or detail toolbars relayout — right edge clips (regression: typing `clip`, `note`).
- Glass, borders, selection, and search chrome go on **pinned child views** (`GeekUIKit` helpers — see [LAUNCHER_PANEL_CONSTRAINTS.md](LAUNCHER_PANEL_CONSTRAINTS.md)).
- After search, results, home, or detail transitions, the host calls `LauncherInPanelLayout.stabilizePanel(from:)` → `LauncherPanel.enforceLockedGeometry()` (re-centers and clamps width to `lockedFrameSize`) plus `stabilizeContentLayout()`.
- Custom module details (Translate, Notes) pin horizontal stacks to the detail container width; crowded toolbars scroll horizontally.
- During detail ↔ list transitions, fading overlays must not block clicks (`ignoresMouseEvents` or `isHidden` until transition completes — see audit L1). Split-mode detail does not fade out the left Open Apps column.
- Bounded widgets (icons, keycaps, table row surfaces, thumbnails) may keep `wantsLayer` on the widget itself.

## Results

- Stable row height.
- 8-10 visible rows in search mode.
- Keep at most 50 ranked results per snapshot.
- Preserve selection by `ResultID` across updates.
- No visible tutorial copy in the launcher.
- Row kinds: actionable (Return ↩), informational (no Return hint).
- Items whose title exactly matches the query receive a ranking boost (+0.30 additive) so precise matches reliably surface first.

## Home — Open Apps + Command Guide (Frozen 2026-07-03)

- **One list section:** running applications ordered by activation recency (left column on empty home).
- **Right pane:** compact module entry table (one primary row per module) **or** module detail — never mirrors the selected app name (ADR-032).
- **Module detail** keeps **Open Apps** visible in the left column.
- **No** setup, recent, continue, or create sections on empty query.
- **No** `+N more` collapse row — all running apps are listed.
- Idle list rows use a **transparent** background; only hover/selection add fill.
- Any typed query returns to **single-column** search results.
- Workbench resume, clipboard transforms, and create suggestions are reached via **search** and **command prefixes**, not home rows.

## Notes detail (Frozen)

Authoritative rules: [NOTES_DETAIL_CONSTRAINTS.md](NOTES_DETAIL_CONSTRAINTS.md).

- **Today** left chip is a quick action (open/create daily note) — not a persistent view.
- **Inbox** appears only on the right panel segment (`Inbox(n)`), not as a left chip.
- **Outline** panel shows the directory tree only — no embedded Recent group.
- **+ Note** / **+ Folder** and `⌘N` / `⌘⇧N` in Tree mode; new notes open in Typora after create.
- `[Tree | Map]` toggles in-panel; create controls and filter hidden in Map mode.
- Notes create/rename/delete belong in detail or `n` commands — not on empty-query home.

## Settings

- SwiftUI is acceptable.
- Include hotkey, modules, permissions, clipboard retention, language, and debug metrics toggles.
