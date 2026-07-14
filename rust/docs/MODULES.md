# Modules

Personal daily-driver status. Prefer honest `unavailable` / `permission_required` /
`not_configured` over empty results.

**Scope:** solo local use — not a public product. Decisions live in [adr/](./adr/).

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
| Notes | `n` / `note` / `notes` | Available — FTS/CJK index; `n new` / `n daily` / `n browse` / `n recent` / `n status` / `n issues` / `n check` / `n reindex`; Hub shortcuts; excludes via `--notes-exclude` | on |
| Quicklinks | `ql` / `quicklinks` | Available — add/overwrite, open, copy URL, delete | on |
| Snippets | `s` / `snip` | Available — search; add/overwrite; copy/paste; delete | on |
| Todo | `t` / `todo` | Gated — EventKit; bare CLI cannot complete Reminders auth | **off** |
| Projects | `p` / `proj` / `project` | Available — scan/open; `proj browse [path]` (relative names resolve under roots) | **off** |
| Kill | `kill` / `quit` / `k` | Process list; quit/force confirm (`luma.kill-process`) | **off** |
| Secrets | `sec` / `secret` / `secrets` | Labels only; Keychain `com.luma.next.secrets`; unlock/copy confirm | **off** |
| Fake | — | Test/demo module for CLI blackbox | **off** |

## Product rules

- UI is terminal CLI/TUI only.
- `bins/luma` is the sole composition root.
- Platform calls stay behind ports.
- Tests must not steal focus (`open`, osascript, AX paste, system clipboard mutation).
- Destructive / Confirm actions require confirm; cancel must be real.
