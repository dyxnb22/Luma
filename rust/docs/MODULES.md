# Modules

Personal daily-driver status. Prefer honest `unavailable` / `permission_required` / `not_configured` over empty results.

**Scope:** solo local use — not a public product. Stub / signed-host modules were removed from the tree.

**Status vs queue:** this file is capability **status** (what works). Optimization work is tracked in [BACKLOG.md](./BACKLOG.md).

## Shell

| Area | Status | Notes |
| --- | --- | --- |
| Doctor / diagnostics | Available | `luma doctor --json`, TUI `:doctor` |
| Config | Available | Versioned settings; `luma config get/set`; TUI Settings (`:settings` / Ctrl-/ → settings); Space toggle persists via `UpdateSettings` CAS |
| Module registry | Available | Manifest + enable/disable; warmup for enabled modules |

## Modules

| Module | Triggers | Status | Default |
| --- | --- | --- | --- |
| Apps | `app` / `apps` | Available — fuzzy + session MRU; launch / reveal / copy path | on |
| Clipboard | `clip` / `cb` | Available — history, pin/unpin, `clip clear`, paste needs AX; Hub pins | on |
| Notes | `n` / `note` / `notes` | Available — FTS/CJK index; `n new` / `n daily` / `n browse` / `n recent` / `n status` / `n issues` / `n check` / `n reindex`; Copy Path | on |
| Quicklinks | `ql` | Available — add/overwrite, open, copy URL, delete | on |
| Snippets | `s` / `snip` | Available — search; add/overwrite; copy/paste; delete | on |
| Todo | `t` / `todo` | Gated — EventKit; permission row when denied | **off** |
| Projects | `proj` | Available — scan/open; `proj browse [path]` drill-down | **off** |
| Kill | `kill` | Process list; quit/force confirm | **off** |
| Secrets | — | Labels only; Keychain namespace `com.luma.next.secrets` | **off** |
| Fake | — | Test/demo | **off** |

## Product rules

- UI is terminal CLI/TUI only.
- `bins/luma` is the sole composition root.
- Platform calls stay behind ports.
- Tests must not steal focus (`open`, osascript, AX paste, system clipboard mutation).
- Destructive / Confirm actions require confirm; cancel must be real.
