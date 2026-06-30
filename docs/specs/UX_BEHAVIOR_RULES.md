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
- If the raw query exactly matches a snippet trigger word (case-insensitive) in global search mode, Return expands and pastes the snippet inline — the panel dismisses without opening Snippets detail.

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
- Items whose title exactly matches the query receive a ranking boost (+0.30 additive) so precise matches reliably surface first.

## Home — Suggested Section

- Maximum 2 continue-flow suggestions and 1 create suggestion (3 total) per Home render.
- `HomeSuggestionMemory` gates items by recency and completion cooldown before they appear.
- Suggestions include: current project context, daily note, top due reminder, clipboard transforms, clipboard-to-note, clipboard-to-snippet, URL-to-quicklink, in-progress Records.

## Settings

- SwiftUI is acceptable.
- Include hotkey, modules, permissions, clipboard retention, and debug metrics toggles.
