# ADR-032: Home Split Layout with Command Guide

## Status

Accepted (2026-07-03)

Supplements ADR-023 (Route C) for empty-query home discoverability.

## Context

Route C froze empty-query home to **Open Apps only** — a calm app switcher without suggestion rows. That reduced clutter but left a discoverability gap: users could not see that Luma exposes module prefixes (`clip`, `n`, `proj`, …) without typing or reading external docs.

The launcher panel also had unused horizontal space on home: app icons and titles occupy far less than the full list row width.

## Decision

On **empty-query home only**, the launcher uses a **two-column layout**:

| Column | Width | Content |
| --- | --- | --- |
| Left | **360 pt** fixed (`homeLeftColumnWidth`) | Open Apps list (unchanged data — still the only home section) |
| Right | Remaining width | Read-only **command guide** (`LauncherHomeGuidePane`) |

**Panel default size** increases to **940 × 760 pt** (was 720 × 680). Responsive clamps: width 720–980, height 640–820.

### Guide pane behavior

- **No selection:** top discoverable commands from `CommandRegistry` (max 8) with description + first help line.
- **Row selected:** contextual copy — Return action, secondary actions, item subtitle when present.
- **Not a second list:** no clickable command rows; keyboard focus stays in search + left list.

### When split is inactive

- Any non-empty search query → single-column results (Route C unchanged).
- `CommandHintBar` under search still handles prefix hints while typing.

### Module detail in split mode

- **All module details** open in the **right column** (`ModuleDetailPresentation.rightColumn`).
- **Left column** keeps the **Open Apps** list visible and interactive (switch/focus apps without closing detail).
- Opening detail refreshes the left column from `OpenAppsHomeProvider` even when detail was entered from a prior search.
- Detail container width is ~540 pt; module views scroll inside fixed panel width per `LAUNCHER_PANEL_CONSTRAINTS`.

### Open Apps chrome

- Hide `· app` trailing label on parent app rows in the narrow left column (redundant in Open Apps section).

## Consequences

Positive:

- Discoverability without re-adding home suggestion / create / continue rows.
- Better use of panel space; app switcher stays primary on the left.
- Reuses existing `CommandDefinition` / `CommandRegistry` data.

Negative:

- ADR-023 "one column" applies to **search results**, not empty home.
- Larger default panel; must keep `LauncherPanel.position(on:)` + `enforceLockedGeometry()` to avoid clip regressions.
- Requires updating frozen panel + home constraint docs.

## Implementation

- `LauncherChromeTokens` — new default size + `homeLeftColumnWidth`.
- `LauncherHomeSplitLayout` — toggles list width vs full-width constraints.
- `LauncherHomeGuidePane` — scrollable guide content.
- `LauncherRootController.syncHomeGuidePane()` — drives visibility + content.

Authoritative freeze: `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md` (dual-column section).

## Non-goals

- No extra home list sections (recent, create, setup).
- No command grid or module cards on home.
- No widening module detail layouts from this change.
