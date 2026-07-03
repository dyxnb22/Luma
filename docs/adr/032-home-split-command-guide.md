# ADR-032: Home Split Layout with Command Guide

## Status

Accepted (2026-07-03)

Supplements ADR-023 (Route C) for empty-query discoverability and split module detail.

## Context

Route C froze empty-query home to **Open Apps only** — a calm app switcher without suggestion rows. That reduced clutter but left a discoverability gap: users could not see that Luma exposes module prefixes (`clip`, `n`, `proj`, …) without typing or reading external docs.

The launcher panel also had unused horizontal space on home: app icons and titles occupy far less than the full list row width.

Opening module detail previously covered the entire content area, hiding Open Apps and breaking the app-switcher mental model.

## Decision

When the **visible search query is empty**, the launcher uses a **two-column layout**:

| Column | Width | Content |
| --- | --- | --- |
| Left | **360 pt** fixed (`homeLeftColumnWidth`) | Open Apps list (unchanged data — still the only home section) |
| Right | Remaining (~540 pt) | Command guide **or** module detail |

**Panel default size** increases to **940 × 760 pt** (was 720 × 680). Responsive clamps: width 720–980, height 640–820.

### Guide pane behavior

- **Always** shows the module command catalog (`CommandRegistry.discoverableCommands`) — title stays **命令 / Commands**.
- **Never** mirrors the left Open Apps row title (e.g. app name) as the guide header.
- Each module block: triggers (`n · note · notes`), description, and up to 3 help lines (e.g. Notes create/search).
- **Not a second list:** no clickable command rows; keyboard focus stays in search + left list.

### When split is inactive

- Any **non-empty** visible search query → single-column results (Route C unchanged).
- `CommandHintBar` under search still handles prefix hints while typing.

### Module detail (split mode)

- **All module details** open in the **right column** only (`ModuleDetailPresentation.rightColumn`).
- **Left column** keeps **Open Apps** visible and interactive (mouse: select app; Return activates app and dismisses panel per leave-launcher rules).
- Opening detail always refreshes the left column from `OpenAppsHomeProvider`, including when entered from a prior search.
- Module detail views scroll inside the right column width; they must not widen the panel.

### Open Apps chrome

- Hide `· app` trailing label on parent app rows in the narrow left column (redundant in Open Apps section).

## Consequences

Positive:

- Discoverability without re-adding home suggestion / create / continue rows.
- App switcher stays available while browsing module detail.
- Reuses existing `CommandDefinition` / `CommandRegistry` data.

Negative:

- ADR-023 "one column" applies to **search results**, not empty-query split.
- Larger default panel; must keep `LauncherPanel.position(on:)` + `enforceLockedGeometry()` to avoid clip regressions.
- Module detail content area is narrower (~540 pt); all detail views must scroll/truncate.

## Implementation

- `LauncherChromeTokens` — default size + `homeLeftColumnWidth`.
- `LauncherHomeSplitLayout` — column constraints; right pane guide vs detail.
- `LauncherHomeGuidePane` — scrollable guide content.
- `LauncherRootController.syncSplitLayout()` — drives column split + right pane mode.
- `LauncherContentCoordinator.present` / `closeDetail` — right-column detail; list stays visible.

Authoritative freeze: `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`.

## Non-goals

- No extra home list sections (recent, create, setup).
- No command grid or module cards on home.
- No full-panel detail overlay while visible query is empty.
