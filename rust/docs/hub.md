# Hub

## Command prefix

Interactive module commands use a leading `/` (`/ssh `, `/rec browse`, `/cmd test`). Local
surfaces also accept `/settings` and `/help`. For first-time setup, use `/settings projects-root PATH`
or `/settings import-project PATH`; Records can be connected with
`/settings records-root PATH`. Input without `/` is treated as a global search. Enter on a bare
slash trigger such as `/clip` commits it as `/clip `.

Empty prompt shows:

1. **Windows** — all visible windows (`title · app`); Enter or **1–9** focuses (status/more/Continue/modules rows are not numbered). Soft-refreshes about every 2s while Hub is visible.
2. **Continue** — up to three live-or-recent objects. Running/paused timers come first, followed
   by privacy-safe Recall objects; Enter runs the natural primary action. Recall contains no raw
   clipboard/snippet body, SSH configuration, or search text.
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
| SSH | `/ssh ` — Host aliases from `~/.ssh/config` |
| Timers | `/tm ` — stopwatch / Pomodoro list |
| Secrets | `/sec ` — vault labels (unlock/copy) |
| Apps / Clipboard / Snippets / Quicklinks / Windows / Proxy | `/app ` / `/clip ` / `/s ` / `/ql ` / `/win ` / `/proxy ` — list dump |

Commands without the `/` prefix are global searches; use `/proj ` or `/clip ` to enter a module.

Clipboard favorites: pin/unpin inside `/clip` (not on Hub).

## Keyboard constraints

- Hub digits `1`–`9` target only visible window rows. Status, overflow, Continue, and module rows have no
  digit and cannot be focused by a digit.
- In `/win`, digits target windows only while `FocusZone::List` is active. When the prompt is
  focused, digits remain search input.
- ActionPicker digit behavior is unchanged; it continues to select actions rather than windows.
