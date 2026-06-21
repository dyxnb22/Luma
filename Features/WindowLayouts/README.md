# Window Layouts

## Goal

Provide Raycast-style window management and convenient page/app splitting: left half, right half, thirds, quarters, maximize, center, and saved layouts.

## MVP Behavior

- Requires Accessibility permission lazily.
- Current implementation applies presets through Accessibility APIs when permission is granted.
- Commands:
  - Move current window left half.
  - Move current window right half.
  - Move current window top/bottom half.
  - Maximize.
  - Center.
  - Apply saved split layout.
- Saved layouts:
  - App A left, App B right.
  - Browser left, editor right.

## UI Card

- Shows current layout preset.
- Edit button opens layout preset manager.
- Drag position persists in dashboard layout.

## Implementation Entry

- Source module: `Sources/LumaModules/WindowLayouts/WindowLayoutsModule.swift`
- Service boundary: `Sources/LumaServices/Accessibility/AXService.swift`
