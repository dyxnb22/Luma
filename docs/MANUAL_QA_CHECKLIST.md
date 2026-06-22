# Manual QA Checklist

## Hotkey

- Works after launch.
- Default chord is Command+Space.
- Toggles panel when panel is visible.
- Still works after sleep/wake.
- Does not collide with Spotlight or input method shortcuts.

## Panel

- Appears on active display.
- Appears over fullscreen apps.
- Appears on all Spaces as expected.
- Query field is focused immediately.
- Escape dismisses.
- Focus loss behavior matches Settings.

## Performance

- Hotkey -> panel p95 <= 50 ms.
- Keystroke -> first result p95 <= 30 ms.
- No visible row jumping while typing.

## Permissions

- Accessibility denied state is clear.
- Accessibility granted state enables window focus.
- Settings links to System Settings when permission is missing.

## Clipboard

- Does not store concealed/transient pasteboard values.
- Skips the entire pasteboard change if any blocked type is present.
- Enforces 500-entry, 7-day, and 100-KB body caps.
- Enforces retention caps.
- Clear history command works.

## Dashboard Widget (Route B)

### Panel Glass

- Liquid glass background visible.
- Top highlight gradient visible.
- 1 pt inner border visible.
- 20 pt continuous corner radius.
- Panel does not bleed background on light desktops.

### Sidebar

- Open Apps section appears with 11 pt uppercase header.
- Up to 10 running apps shown.
- Frontmost app row has accent background.
- Order changes after switching apps externally.
- Sidebar does not list Luma itself.
- Clicking a sidebar row activates that app.

### Feature Grid

- Four widget cards render in this order: Translate, Clipboard, Calculator, Windows.
- Each card has top-bright-to-bottom-dark gradient.
- Hover scales card to 1.04 smoothly (no jump).
- Press scales card to 0.96.
- Command+1...4 opens corresponding card.

### Search Results

- Typing fades feature grid out and results list in within ~80 ms.
- Clearing text restores the feature grid.
- Up/Down moves selection without rebuilding rows.
- Selected row shows accent background and `↩`.
- Return runs the selected item and hides the panel.
- Selection sticks to the previously selected item if it remains in the new result set.

### Module Detail

- Clicking a card swaps the center area to the module detail view.
- Sidebar and search bar remain visible.
- Top bar shows Back, module title, and ✕.
- Esc in detail returns to the feature grid (does not close the panel).
- Esc again on the grid closes the panel.
- Quickly opening card A → Esc → card B leaves card B's detail showing (no view residue).

### Hotkey Status

- Menu bar shows ⌘ icon when hotkey is registered.
- Menu bar shows triangle icon plus "Disable Spotlight's ⌘+Space" entry when registration fails.
