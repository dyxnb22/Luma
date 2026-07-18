# Modules

The optional `luma-menubar` companion is not a module and does not participate in the module
registry. It provides only Wordbook glance status, visible-window focus, and entry points into
the terminal TUI. See [ADR-0006](./adr/0006-native-menubar-companion.md).

Personal daily-driver status. Prefer honest `unavailable` / `permission_required` /
`not_configured` over empty results.

**Scope:** solo local use — not a public product. Decisions live in [adr/](./adr/).

Interactive module commands require a leading `/` (for example `/ssh`, `/rec browse`, and
`/settings`). Bare trigger text is treated as a global search.

## Shell

| Area | Status | Notes |
| --- | --- | --- |
| Doctor / diagnostics | Removed | Centralized doctor removed; modules still surface permission/unavailable/not_configured |
| Config | Available | Versioned settings; `luma config get/set`; TUI Settings via `/settings`; Ctrl-/ opens command palette; Space toggle persists via `UpdateSettings` CAS; project imports and `records_root` use settings CAS. `enabled_modules` keys are **sticky** by module id string — renaming a module id does not migrate or delete the old key; stale entries remain until cleaned by hand |
| Module registry | Available | Manifest + enable/disable; warmup for enabled modules |

## Modules

| Module | Triggers | Status | Default |
| --- | --- | --- | --- |
| Apps | `/app` / `/apps` | Available — fuzzy + session MRU; launch / reveal / copy path | on |
| Windows | `/win` / `/window` / `/windows` | Available — list+focus; Hub 1–9 quick focus; `/win` digits only when List is focused; prompt digits are preserved; hard cap 15 | on |
| Proxy | `/proxy` / `/px` | Available — controller-first Mihomo status, groups/nodes, mode, local macOS HTTP/SOCKS proxy controls, and safe Luma Profile import/list/use/delete/refresh; Clash Verge Profiles are read-only unless Luma-owned. See [Proxy](./PROXY.md). | on |
| Clipboard | `/clip` / `/cb` | Available — history, pin/unpin, `/clip clear`, paste needs AX; soft cap **500** unpinned rows; entries over **256 KiB** rejected | on |
| Notes | `/n` / `/note` / `/notes` | Available — FTS/CJK index; `/n new` / `/n daily` / `/n browse` / `/n recent` / `/n status` / `/n issues` / `/n check` / `/n reindex`; excludes via `--notes-exclude`; workspace I/O is adapter-backed with bounded, non-symlink previews/creation | on |
| Quicklinks | `/ql` / `/quicklinks` | Available — add/overwrite, open, copy URL, delete | on |
| Snippets | `/s` / `/snip` | Available — search; add/overwrite; copy/paste; delete | on |
| Wordbook | `/wb` / `/wordbook` / `/words` | Available — due/new/wrong lists; `/wb review due\|new\|wrong` one-word session; Enter/Space reveal, 1/2/3 grade, m mastered with confirmation, s skip, Esc exit; queue uses remaining daily goal; `/wb import PATH` accepts a regular non-symlink UTF-8 CSV up to 512 KiB; daily goal. Search/perform (import, speak, pasteboard) honor cancel tokens | on |
| Records | `/rec` / `/record` | Available — SQLite-backed media log; `/rec <query>` / `/rec browse`; `/rec add`, `/rec rate`, `/rec note`, ActionPicker edit/remove; CLI also has `record import`, `import-status`, `backup`; Markdown import is dry-run by default and `--apply` is ledger-backed with a LumaNext backup, source Markdown stays read-only | on |
| Projects | `/p` / `/proj` / `/project` | Available — only manually imported projects appear in plain search; `/proj add/import PATH`, `/proj remove NAME\|PATH`, `/proj browse`; canonical existing non-symlink paths, duplicate rejection, config-only removal | on |
| Command Recipes | `/cmd` / `/recipe` / `/recipes` | Available — semantic templates with project variants; ordered `program + args`; TUI runs in current terminal; user TOML + built-ins. See [Command Recipes](./COMMAND_RECIPES.md). | on |
| SSH | `/ssh` | Available — reads `~/.ssh/config` Host aliases; `/ssh fav` / `/ssh recent` / `/ssh rename`; favorite/recent metadata in `ssh_meta.sqlite`; Enter connects in current terminal; SFTP + copy alias actions. See [SSH](./SSH.md). | on |
| Timers | `/tm` / `/timer` / `/timers` | Available — stopwatch + countdown/Pomodoro; `/tm pomo [min] [name]`, `/tm sw [name]`, `/tm 25`; start/pause/resume/reset/delete; state in `timers.sqlite`; speech alert on completion while Luma is running (no daemon — quitting pauses running timers). In-process 1s poller cancels on teardown; search/perform honor cancel | on |
| Secrets | `/sec` / `/secret` / `/secrets` | Copy-only for pre-provisioned labels; `luma secrets set` bootstrap; unlock is in-process UX only (no Touch ID); copy confirm | **off** (enable in Settings after bootstrap) |
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

### SSH Connections

Read-only launcher over OpenSSH — not a full SSH client:

- **Config:** `~/.ssh/config` (concrete `Host` aliases; `Include` depth 8; wildcard patterns skipped). Override with `SSH_CONFIG` for tests.
- **Metadata:** `~/Library/Application Support/LumaNext/ssh_meta.sqlite` — favorites, local display names, `last_connected_at`, `connection_count`. Luma does not write back to `~/.ssh/config`.
- **Resolve:** macOS adapter runs `ssh -G <alias>` (cached per session; `/ssh reload` clears cache).
- **Connect:** TUI suspends → `ssh <alias>` or `sftp <alias>` in the current terminal → resume. Successful exit (`0`) records connection metadata.
- **Queries:** `/ssh `, `/ssh <needle>`, `/ssh fav`, `/ssh recent`, `/ssh reload`, `/ssh rename ALIAS NAME` (case-insensitive `rename` prefix; name may contain spaces).
- **CLI:** `luma ssh list|connect|sftp|favorite|unfavorite|rename`.
- **Search honesty:** missing config → `not_configured`; parse or `ssh` binary errors → `unavailable`. Preview never shows private key contents.
- **Details:** [SSH.md](./SSH.md).

### Timers

In-session stopwatch and countdown (Pomodoro) — no background daemon:

- **Store:** `~/Library/Application Support/LumaNext/timers.sqlite`
- **Queries:** `/tm ` lists timers; `/tm pomo [minutes] [name]`, `/tm 25`, `/tm sw [name]` / `/tm start [name]` create+start rows.
- **Actions:** Start / Pause / Resume / Reset; Delete (confirm).
- **Alerts:** speech (“… done”) when a countdown finishes **while Luma is running**. Quitting pauses running timers so elapsed time does not advance silently offline.
- **Concurrency / cancel:** warmup starts a session-scoped 1s poller; teardown cancels it and bumps a generation so in-flight ticks cannot alert after shutdown. Search and perform return early when their cancel token fires.
- **Honesty:** store/clock failures surface as `unavailable` rows.

### Wordbook (concurrency)

- Search and perform check cancel before mutating or speaking.
- Import / pasteboard / speech paths use cancel-aware awaits so Esc / superseded ops do not leave half-applied UI side effects.
- Review queue load is engine-owned (`LoadWordbookReview`); grading still goes through normal ExecuteAction cancel.

### Clipboard capacity

Aligned with `luma-storage` clipboard store constants:

- **500** unpinned history rows (soft cap; pinned rows are never evicted by this cap).
- **256 KiB** max bytes per entry (`MAX_ENTRY_BYTES`); larger pastes are rejected.

## Product rules

- The full workbench UI is terminal CLI/TUI; the optional menu bar is glance-only and not a
  module surface.
- `bins/luma` is the sole module-registration composition root.
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
  import CAS and path validation, Records parser edge cases, SSH config parse and metadata
  round-trips, interactive-terminal contract, and CLI dry-run/apply behavior.
