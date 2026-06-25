# Window Layouts

## Goal

Command-first window positioning for the focused window: left/right/top/bottom half, maximize, and center. Not a full window manager — no thirds, saved multi-app splits, or dashboard UI in v1.

## Active Behavior (Route C)

- Registered in `BuiltInModules.makeAll()`.
- Prefix triggers only: `layout`, `win`, `wl` (e.g. `layout left`, `win center`).
- Empty payload lists all presets; payload filters by title/alias.
- Return applies preset to the **focused window** via Accessibility APIs.
- Without Accessibility permission: returns a single “Grant Accessibility Permission” result (no silent failure).
- No home-screen row; not on empty-query home.

## Commands (v1)

- Left Half, Right Half, Top Half, Bottom Half
- Maximize, Center

## Implementation Entry

- Module: `Sources/LumaModules/WindowLayouts/WindowLayoutsModule.swift`
- Engine: `Sources/LumaModules/WindowLayouts/WindowLayoutEngine.swift`
- Service: `Sources/LumaServices/Accessibility/AXService.swift` (`applyWindowLayout`)

## Out of Scope (v1)

- Left/right thirds, restore previous frame
- Screen containing focused window (uses main screen; TODO in AXService)
- Dashboard card / layout preset manager UI
