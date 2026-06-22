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

- Seven widget cards render: Translate, Clipboard, Notes, Todo, Wordbook, Snippets, Secrets (row-major order).
- Each card has appearance-aware top-bright-to-bottom-dark gradient (light and dark desktops).
- Hover scales card smoothly; press scales down.
- Command+1…7 opens the corresponding card when visible.

### Search Results

- Typing fades feature grid out and results list in within ~80 ms.
- Clearing text restores the feature grid.
- Up/Down moves selection without rebuilding rows.
- Selected row shows accent background and Return hint capsule.
- Return runs the selected item and hides the panel.
- Selection sticks to the previously selected item if it remains in the new result set.

### Module Detail

- Clicking a card swaps the center area to a `BaseDetailContainer` detail view (16 pt margins).
- Sidebar and search bar remain visible.
- Top bar shows Back, module title, and close.
- Esc in detail returns to the feature grid (does not close the panel).
- Esc again on the grid closes the panel.

### Latency HUD

- Settings → General → "Show latency HUD overlay" toggles bottom-right p95 overlay.
- Overlay updates after each search paint when enabled.

### Events (Calendar)

- `e` lists today's calendar events (requires Calendar permission).
- `e meet john tomorrow 14:00` previews create; Return saves to Calendar.
- `e ?` shows help lines.

### Apps RAM

- `app top` lists running apps by resident memory; Return activates; secondary action quits app.

### Clipboard → Snippet

- Clipboard detail row action saves entry via snippet editor sheet.

### Settings Activity

- Settings → Activity tab shows 7-day and 30-day sparklines plus per-module counts.

### Module Help

- `?` / `help` on module triggers: `t ?`, `s ?`, `m ?`, `secret ?`, `clip ?`, `tr ?`, `e ?`, `app ?`, and global `?` for commands.

### Hotkey Status

- Menu bar shows ⌘ icon when hotkey is registered.
- Menu bar shows lock icon when secrets vault is locked; open lock when unlocked.
- Menu bar shows triangle icon plus "Disable Spotlight's ⌘+Space" entry when registration fails.

## Snippets

- Dashboard green card opens detail view.
- Add snippet via detail → `s <query>` finds it → Return copies content.
- Tab on a snippet result pastes into front app (requires Accessibility).
- Edit / Delete / Duplicate round-trip across app restart.

## Secrets

- Dashboard gold card opens detail view.
- Locked state shows Unlock button; unlocked shows CRUD table.
- Add secret persists across restart; value only in Keychain.
- `secret unlock` then `secret <label>` lists secrets in launcher.
- Copy secret → pasteboard cleared after 10 s if not overwritten.
- After 5 min idle, vault auto-locks; menu bar icon updates.
- Settings: auto-clear, re-lock timeout, require-unlock-on-launch.

## Media

- `m Oppenheimer movie 9` captures and appears in `m oppen` search.
- `m` shows recent items; `m log` opens detail view.
- Detail view: category tabs, status filter, sort work on empty and populated lists.
- Add / Edit / Delete round-trip across restart.
- Export CSV writes to Downloads and opens in Finder.
- Tab on search result copies `title — rating/10`.

## Notes v0.1

- Root picker writes `notes.json`.
- Launcher hit: type `tree`, Return opens `Tree.md` in Typora.
- Detail tree loads, default expansion is root only.
- Create / rename / delete (note + empty folder) round-trip.
- Non-empty folder delete is refused with the expected message.
- External `mkdir` in the root surfaces in the tree within 1 second.
- Image tools panel: scan + migrate + Typora config check.
- Typora not installed: open falls back to `NSWorkspace.open` with no prompt.
