# UX Behavior Rules

## Launcher

- Hotkey toggles the launcher.
- Default hotkey is Command+Space.
- The launcher panel is not a dashboard.
- Module detail surfaces open in separate windows, not inside the launcher panel.
- Escape dismisses.
- Return runs the selected primary action.
- Command+Return runs the first secondary action when present.
- Arrow up/down moves selection.
- Command+number selects or runs the visible nth result.
- Panel dismisses immediately after action dispatch.
- Results update progressively as modules return.
- Empty query shows the top 8 recent/frequent items.

## Panel

- AppKit `NSPanel`, borderless, floating, pre-instantiated.
- Shows across Spaces and fullscreen apps.
- Positioned in the upper third of the active screen.
- No scale animation; optional <= 60 ms fade/translate.
- The query field receives focus on every show.

## Results

- Stable row height.
- 8-10 visible rows.
- Keep at most 50 ranked results per snapshot.
- Preserve selection by `ResultID` across updates.
- No visible tutorial copy in the launcher.

## Settings

- SwiftUI is acceptable.
- Include hotkey, modules, permissions, clipboard retention, and debug metrics toggles.
