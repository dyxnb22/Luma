# Hub

Empty prompt shows:

1. **Windows** — all visible windows (`title · app`); Enter or **1–9** focuses (status/more/modules rows are not numbered). Soft-refreshes about every 2s while Hub is visible.
2. **Modules** — Enter opens each module’s `suggested_query` (its default surface).

## Module defaults (Hub Enter)

| Module | Opens |
|--------|--------|
| Notes | `n ` — directory tree at notes root (`n recent` = recent flat list) |
| Projects | `proj browse` — browse project roots (import from browse) |
| Wordbook | `wb due` — due words; use `wb review due` for the review session |
| Records | `rec ` — categories or imported media records |
| Secrets | `sec ` — vault labels (unlock/copy) |
| Apps / Clipboard / Snippets / Quicklinks / Windows | `app ` / `clip ` / `s ` / `ql ` / `win ` — list dump |

Bare triggers without a trailing space (`n`, `clip`) do not search — add a space to enter the module.

Notes index issues: status row with `errors N` → Enter opens `n issues`; issue rows Open / copy path.

Clipboard favorites: pin/unpin inside `clip` (not on Hub).

## Keyboard constraints

- Hub digits `1`–`9` target only visible window rows. Status, overflow, and module rows have no
  digit and cannot be focused by a digit.
- In `win`, digits target windows only while `FocusZone::List` is active. When the prompt is
  focused, digits remain search input.
- ActionPicker digit behavior is unchanged; it continues to select actions rather than windows.
