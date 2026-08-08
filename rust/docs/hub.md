# Hub

## Command prefix

Interactive module commands use a leading `/` (`/proj `, `/rec browse`, `/cmd test`). Local
surfaces also accept `/settings` and `/help`. For first-time setup, use `/settings projects-root PATH`
or `/settings import-project PATH`; Records can be connected with
`/settings records-root PATH`. Input without `/` is treated as a global search. Enter on a bare
slash trigger such as `/clip` commits it as `/clip `.
`Ctrl-/` or `/commands [filter]` opens the command palette. Enabled module manifests supply its
subcommands, parameter placeholders, examples, `/help` entries, and partial-command candidates.

Empty prompt shows:

1. **Windows** — all visible windows (`title · app`); Enter or **1–9** focuses (status/more/Continue/modules rows are not numbered). Soft-refreshes about every 2s while Hub is visible.
2. **Continue** — up to three privacy-safe Recall objects that their owning module can re-read and
   validate; Enter runs the current natural primary action. Stale/deleted objects are pruned.
   Recall contains no raw clipboard body or search text.
3. **Modules** — Enter opens each module’s `suggested_query` (its default surface).

## Module defaults (Hub Enter)

| Module | Opens |
|--------|--------|
| Projects | `/proj ` — recall-ranked imported projects; Enter opens `/proj show PATH` |
| Git | `/git` — imported-project dashboard; Enter on a repository opens its workbench |
| Runtime | `/run` — current local TCP listeners |
| Wordbook | `/wb review` — today's due-first review queue, filled with new words to the remaining goal |
| Records | `/rec ` — categories or imported media records |
| Command Recipes | `/cmd ` — local command recipes and project variants |
| Apps / Clipboard / Windows | `/app ` / `/clip ` / `/win ` — list dump |

Commands without the `/` prefix are global searches; use `/proj ` or `/clip ` to enter a module.

Clipboard favorites: pin/unpin inside `/clip` (not on Hub).

## Keyboard constraints

- Hub digits `1`–`9` target only visible window rows. Status, overflow, Continue, and module rows have no
  digit and cannot be focused by a digit.
- In `/win`, digits target windows only while `FocusZone::List` is active. When the prompt is
  focused, digits remain search input.
- ActionPicker digit behavior is unchanged; it continues to select actions rather than windows.
- `⌥↑` / `⌥↓` page the Hub without opening the selected row (`fn↑` / `fn↓` is a
  compact-Mac compatibility alias). From the Hub, `Ctrl-/` exposes
  `/scroll up` and `/scroll down`; selecting either returns to the Hub and performs the same
  reducer-only movement without a Hub refresh or module I/O.
- Mouse reporting is not enabled. Click/drag remains available to the terminal host for ordinary
  text selection.
