# Hub

Empty prompt shows:

1. **Windows** — all visible windows (`title · app`); Enter focuses (hard cap 15; overflow → `win `). Soft-refreshes about every 2s while Hub is visible.
2. **Modules** — Enter opens each module’s `suggested_query` (its default surface).

## Module defaults (Hub Enter)

| Module | Opens |
|--------|--------|
| Notes | `n ` — directory tree at notes root (`n recent` = recent flat list) |
| Projects | `proj browse` — project tree |
| Secrets | `sec ` — vault labels (unlock/copy) |
| Apps / Clipboard / Snippets / Quicklinks / Windows | `app ` / `clip ` / `s ` / `ql ` / `win ` — list dump |

Bare triggers without a trailing space (`n`, `clip`) do not search — add a space to enter the module.

Notes index issues: status row with `errors N` → Enter opens `n issues`; issue rows Open / copy path.

Clipboard favorites: pin/unpin inside `clip` (not on Hub).
