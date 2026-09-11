# Keyboard contract (macOS)

Luma is designed for a compact Mac keyboard first. A shortcut shown in the UI must be physically
available without an extended keyboard, must not depend on an F-key media-mode setting, and must
have the same meaning in code, `/help`, the footer, and current documentation.

## Canonical shortcuts

| Scope | Shortcut | Meaning |
| --- | --- | --- |
| Native host | `⌥Space` | Show/hide Luma on a fresh install |
| Global TUI | `Ctrl-/` | Open the local command palette |
| Scrollable local surface | `⌥↑` / `⌥↓` | Move backward/forward one viewport |
| Search results | `Ctrl-k` | Open actions for the selected result |
| Search prompt | `Ctrl-p` / `Ctrl-n` | Older/newer query |
| SSH terminal | `Ctrl-/`, then `c` | Show or focus the SSH command shelf |
| SSH terminal | `Ctrl-/`, then `f/d/r/q` | Fullscreen, disconnect, reconnect, or leave |
| SSH terminal | `⌥↑` / `⌥↓` | Browse Luma's local SSH scrollback |

The host accepts `fn↑` / `fn↓` as a compatibility alias for viewport movement because compact Mac
keyboards emit those gestures as terminal function-key events. The alias may be documented in
long-form help, but UI hints use the canonical `⌥↑` / `⌥↓` shortcut. Luma never requires or labels
dedicated Page Up, Page Down, Home, End, Insert, or function keys.

`Ctrl-/` has one product meaning: open local commands. Inside SSH it opens a short command layer so
the remote terminal retains ordinary letters. Pressing `Space` in that layer sends the raw
`Ctrl-/` byte to the remote program. `Ctrl+Space` and `F1`–`F12` are not Luma shortcuts and are
forwarded when macOS delivers them.

## Scoped single-key actions

Unmodified letters and digits are intercepted only while their owning local surface has focus:

- ActionPicker and the Windows/Hub list use visible `1`–`9` rows.
- Wordbook Review uses `1`/`2`/`3`, `m`, and `s`.
- Command Recipes rows use `r`, `c`, and `f`; Git rows use their displayed list actions.
- The SSH command shelf uses `c`, `i`, and `f` only after the shelf has focus.

Prompt input and pasted text never pass through these shortcut tables.

## Host activation conflicts

Any global shortcut can be claimed by another launcher. Fresh installs prefer `⌥Space`, which has
no default macOS system binding. A previously saved choice remains unchanged. Registration is
checked at startup; if it fails, Luma offers `⌘⇧Space` and `⌘Space` explicitly and never changes the
saved shortcut silently. `⌘Space` is last because macOS normally reserves it for Spotlight.

## Maintenance rule

Code may accept terminal compatibility events, but user-facing strings must name the canonical Mac
gesture. New bindings must be added to the central input mapper or the scoped module-shortcut
table, covered by a mapping test, and reflected here before they appear in a footer or document.
