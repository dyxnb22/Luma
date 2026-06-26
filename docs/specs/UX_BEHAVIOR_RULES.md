# UX Behavior Rules

## Launcher

- Hotkey toggles the launcher.
- Default hotkey is Command+Space.
- The launcher panel is not a dashboard.
- Module detail surfaces open inside the launcher panel (Route C).
- Escape dismisses, or steps back through detail → results → home.
- Return runs the selected row's primary action.
- Tab opens the action panel for secondary actions; Tab again closes it.
- Command+Return runs the first secondary action when present.
- Arrow up/down moves selection.
- Command+number selects or runs the visible nth result (unified across home and results).
- Panel dismisses immediately after immediate (leave-launcher) actions.
- In-panel actions (open detail, replace query, translate) keep the panel visible.
- Results update progressively as modules return.
- Empty query shows the home screen: Open Apps and Suggested sections.

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
- Row kinds: actionable (Return ↩), starter (→), informational (no Return hint).

## Settings

- SwiftUI is acceptable.
- Include hotkey, modules, permissions, clipboard retention, and debug metrics toggles.
