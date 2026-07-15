# Modules

Personal daily-driver status. Prefer honest `unavailable` / `permission_required` /
`not_configured` over empty results.

**Scope:** solo local use — not a public product. Decisions live in [adr/](./adr/).

## Shell

| Area | Status | Notes |
| --- | --- | --- |
| Doctor / diagnostics | Removed | Centralized doctor removed; modules still surface permission/unavailable/not_configured |
| Config | Available | Versioned settings; `luma config get/set`; TUI Settings via `:settings`; Ctrl-/ opens command palette; Space toggle persists via `UpdateSettings` CAS; project imports and `records_root` use settings CAS |
| Module registry | Available | Manifest + enable/disable; warmup for enabled modules |

## Modules

| Module | Triggers | Status | Default |
| --- | --- | --- | --- |
| Apps | `app` / `apps` | Available — fuzzy + session MRU; launch / reveal / copy path | on |
| Windows | `win` / `window` / `windows` | Available — list+focus; Hub 1–9 quick focus; `win` digits only when List is focused; prompt digits are preserved; hard cap 15 | on |
| Proxy | `proxy` / `px` | Available — controller-first Mihomo status, groups/nodes, mode, local macOS HTTP/SOCKS proxy controls, and safe Luma Profile import/list/use/delete; Clash Verge Profiles are read-only unless Luma-owned | on |
| Clipboard | `clip` / `cb` | Available — history, pin/unpin, `clip clear`, paste needs AX | on |
| Notes | `n` / `note` / `notes` | Available — FTS/CJK index; `n new` / `n daily` / `n browse` / `n recent` / `n status` / `n issues` / `n check` / `n reindex`; excludes via `--notes-exclude` | on |
| Quicklinks | `ql` / `quicklinks` | Available — add/overwrite, open, copy URL, delete | on |
| Snippets | `s` / `snip` | Available — search; add/overwrite; copy/paste; delete | on |
| Wordbook | `wb` / `wordbook` / `words` | Available — due/new/wrong lists; `wb review due\|new\|wrong` one-word session; Enter/Space reveal, 1/2/3 grade, m mastered with confirmation, s skip, Esc exit; queue uses remaining daily goal; import and daily goal | on |
| Records | `rec` / `record` | Available — SQLite-backed media log; `rec <query>` / `rec browse`; `rec add`, `rec rate`, `rec note`, ActionPicker edit/remove; CLI also has `record import`, `import-status`, `backup`; Markdown import is dry-run by default and `--apply` is ledger-backed with a LumaNext backup, source Markdown stays read-only | on |
| Projects | `p` / `proj` / `project` | Available — only manually imported projects appear in plain search; `proj add/import PATH`, `proj remove NAME\|PATH`, `proj browse`; canonical existing non-symlink paths, duplicate rejection, config-only removal | on |
| Secrets | `sec` / `secret` / `secrets` | Copy-only for pre-provisioned labels; `luma secrets set` bootstrap; unlock is in-process UX only (no Touch ID); copy confirm | **off** (enable in Settings after bootstrap) |
| Fake | — | Test/demo module for CLI blackbox | **off** |

### Secrets Keychain bootstrap

No provisioning UI. Labels come from a sidecar plus Keychain entries:

- **Service:** `com.luma.next.secrets`
- **Sidecar:** `~/Library/Application Support/LumaNext/secrets-labels.json` (label list only; no values)
- **Add a secret (CLI):**
  ```bash
  printf '%s' 'your-secret-value' | luma secrets set api-token
  ```
  Reads the value from **stdin** (never argv). The macOS adapter writes Keychain and appends the account to the sidecar.
- **Enable module:** `luma config set --enable-module luma.secrets` (default-off until labels exist).
- **Search honesty:** empty labels → `not_configured` row with bootstrap hint; sidecar/keychain errors → `unavailable`; values never appear in search (copy-only after unlock + confirm).
- **Unlock:** in-process session gate only — not Touch ID, Keychain ACL, or an OS auth prompt. Locks on teardown/exit and after idle (`secrets_idle_lock_secs`, default 300; `0` disables).

## Product rules

- UI is terminal CLI/TUI only.
- `bins/luma` is the sole composition root.
- Platform calls stay behind ports.
- Tests must not steal focus (`open`, osascript, AX paste, system clipboard mutation).
- Destructive / Confirm actions require confirm; cancel must be real.
- There is no centralized `luma doctor`, `:doctor`, Doctor overlay, diagnostics export, or
  probe-port workflow. Modules own their `permission`, `unavailable`, and `not_configured` rows.
- Project import mutations go through the application settings CAS; modules do not write
  `ConfigStore` directly. Removing a project only edits settings and never deletes its directory.
- Records use `records.sqlite` as the long-term source of truth after import. Imported Markdown is
  read-only; import is idempotent, DB edits win over changed source rows, and migration rollback
  restores only the artifact belonging to that migration kind.
- Tests cover prompt digit routing, window row hints, review reveal/grade/confirmation/exit,
  import CAS and path validation, Records parser edge cases, and CLI dry-run/apply behavior.
