# Modules

The optional native workbench host is not a module and does not participate in the module
registry. It only hosts the Rust TUI in a PTY and provides global activation. See
[ADR-0007](./adr/0007-native-workbench-host.md).

Personal daily-driver status. Prefer honest `unavailable` / `permission_required` /
`not_configured` over empty results.

**Scope:** solo local use — not a public product. Decisions live in [adr/](./adr/).

Interactive module commands require a leading `/` (for example `/ssh`, `/rec browse`, and
`/settings`). Bare trigger text is treated as a global search.

## Shell

| Area | Status | Notes |
| --- | --- | --- |
| Doctor / diagnostics | Removed | Centralized doctor removed; modules still surface permission/unavailable/not_configured |
| Config | Available | Versioned settings; `luma config get/set`; TUI Settings via `/settings`; `/settings notes-root PATH`, `/settings projects-root PATH`, and `/settings import-project PATH` make first-time module setup actionable in the workbench; Ctrl-/ opens command palette; Space toggle persists via `UpdateSettings` CAS. `enabled_modules` keys are **sticky** by module id string — renaming a module id does not migrate or delete the old key; stale entries remain until cleaned by hand |
| Module registry | Available | Manifest + enable/disable; warmup for enabled modules |

## Modules

| Module | Triggers | Status | Default |
| --- | --- | --- | --- |
| Apps | `/app` / `/apps` | Available — fuzzy + session MRU; launch / reveal / copy path | on |
| Windows | `/win` / `/window` / `/windows` | Available — list/search works without Accessibility; focus and Hub focus report AX permission locally; Hub 1–9 quick focus; prompt digits are preserved; hard cap 15 | on |
| Git | `/git` | Available — imported projects only; bounded repository discovery; dashboard prioritizes conflict/dirty/ahead; safe stage/unstage, confirmed tracked-file discard, local log/branches/commit. No remote, fetch, push, pull, clone, rebase, reset, or `git clean`. | on |
| Runtime | `/run` / `/ports` | Available — on-demand local TCP listener list with project association; copy fields and guarded same-user, identity-rechecked SIGTERM only. No background monitor or SIGKILL. | on |
| Proxy | `/proxy` / `/px` | Available — controller-first Mihomo status, groups/nodes, mode, `/proxy status` and on-demand `/proxy check`, local macOS HTTP/SOCKS proxy controls, and safe Luma Profile import/list/use/delete/refresh; HTTPS is reported read-only. Clash Verge Profiles are read-only unless Luma-owned. See [Proxy](./PROXY.md). | on |
| Clipboard | `/clip` / `/cb` | Available — history, pin/unpin, `/clip clear`, paste needs AX; caps: **500** unpinned, **100** pinned; entries over **256 KiB** rejected | on |
| Notes | `/n` / `/note` / `/notes` | Available — FTS/CJK index; configure first use with `/settings notes-root PATH`; `/n new` / `/n daily` / `/n browse` / `/n recent` / `/n status` / `/n issues` / `/n check` / `/n reindex`; excludes via `--notes-exclude`; workspace I/O is adapter-backed with bounded, non-symlink previews/creation | on |
| Quicklinks | `/ql` / `/quicklinks` | Available — add/overwrite, open, copy URL, delete; hard cap **1000** entries (updates remain allowed at capacity) | on |
| Snippets | `/s` / `/snip` | Available — search/add/overwrite/copy/delete without Accessibility; paste reports AX permission locally; hard cap **1000** entries | on |
| Wordbook | `/wb` / `/wordbook` / `/words` | Available — due/new/wrong lists; `/wb review due\|new\|wrong` one-word session; Enter/Space reveal, 1/2/3 grade, m mastered with confirmation, s skip, Esc exit; queue uses remaining daily goal; `/wb import PATH` accepts a regular non-symlink UTF-8 CSV up to 512 KiB; daily goal. Search/perform (import, speak, pasteboard) honor cancel tokens | on |
| Records | `/rec` / `/record` | Available — SQLite-backed media log; `/rec <query>` / `/rec browse`; `/rec add`, `/rec rate`, `/rec note`, ActionPicker edit/remove; CLI also has `record import`, `import-status`, `backup`; Markdown import is dry-run by default and `--apply` is ledger-backed with a LumaNext backup, source Markdown stays read-only | on |
| Projects | `/p` / `/proj` / `/project` | Available — only manually imported projects appear in plain search; `/settings import-project PATH` or `/proj add/import PATH`, `/proj remove NAME\|PATH`, `/proj browse`; canonical existing non-symlink paths, duplicate rejection, config-only removal | on |
| Command Recipes | `/cmd` / `/recipe` / `/recipes` | Available — semantic templates with project variants; ordered `program + args`; TUI runs in current terminal; user TOML + built-ins. See [Command Recipes](./COMMAND_RECIPES.md). | on |
| SSH | `/ssh` | Available — reads `~/.ssh/config` Host aliases; `/ssh fav` / `/ssh recent` / `/ssh rename`; favorite/recent metadata in `ssh_meta.sqlite` with a **1000-row** cap; Enter connects in current terminal; SFTP + copy alias actions. See [SSH](./SSH.md). | on |
| Timers | `/tm` / `/timer` / `/timers` | Available — stopwatch + countdown/Pomodoro; `/tm pomo [min] [name]`, `/tm sw [name]`, `/tm 25`; start/pause/resume/reset/delete; state in `timers.sqlite`, hard cap **256**; speech alert on completion while Luma is running (no daemon — graceful quitting pauses running timers). In-process 1s poller cancels on teardown; search/perform honor cancel | on |
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

### Continue and global recall

- Global search contributors are Apps, Windows, Projects, Notes, Command Recipes, SSH, Clipboard,
  Snippets, Quicklinks, and Git. Results are capped at 12 per module and 60 total, then selected
  round-robin so one large catalogue cannot fill the first page. Records and Wordbook remain
  targeted-only because their dense historical rows lack a clear bounded global-search benefit.
- A successful action records only bounded recall metadata in `recall.sqlite`: object/module/kind,
  natural primary action, a safe display title, optional project association, count, and last use.
  Failed/cancelled actions are not recorded. Clipboard bodies, snippet bodies, SSH configuration,
  proxy endpoints, and search text are never copied into Recall.
- The empty Hub may render at most five compatible Continue rows after Windows. Git and Runtime
  actions remain recall-ranked in global search but are not Hub-continued because their live
  repository/listener payload must be revalidated.

### Git Workbench

- `/git`, `/git dirty|conflict|ahead|behind|clean`, `/git repo PATH`, `/git branches PATH`,
  `/git log PATH` and `/git commit MESSAGE` are local-only
  surfaces. Process calls use explicit `git` arguments, noninteractive environment, timeout, and
  bounded discovery/log/diff presentation.
- Discard uses `git restore --worktree -- PATH` only for tracked, non-conflicted unstaged changes
  and only after confirmation, preserving any version already in the index. It never runs
  `git clean`. Path traversal, NUL, absolute paths, dirty branch switching, and empty commit
  messages are rejected.

### Runtime Console

- `/run` / `/ports` runs `lsof` on demand only. Each listener shows port/address/PID/process/user/
  cwd plus imported project association. No listener information is persisted.
- Termination is a confirmed SIGTERM against a freshly re-listed matching PID identity, only for
  the current user and never for protected system process names. Permission and unavailable rows
  remain local to the module.

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
- **100** pinned rows (hard cap; unpin one before pinning another; pinned data is never silently
  deleted).
- **256 KiB** max bytes per entry (`MAX_ENTRY_BYTES`); larger pastes are rejected.

## Product rules

- The full workbench UI is the Rust CLI/TUI; the optional native host owns only its window, PTY,
  activation, and lifecycle.
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
