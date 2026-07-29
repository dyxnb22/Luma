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
| Config | Available | Versioned settings; `luma config get/set`; TUI Settings via `/settings`; commands cover project import/root, Records root, Clipboard retention, Secrets idle lock, and Hub window count. Ctrl-/ and `/commands [filter]` open a task-grouped searchable palette generated from enabled module `CommandSpec` descriptions; the same descriptions drive `/help` and slash completion. Space toggle persists via `UpdateSettings` CAS. `enabled_modules` keys are **sticky** by module id string — renaming a module id does not migrate or delete the old key; stale entries remain until cleaned by hand |
| Keyboard navigation | Available | Arrow keys move one row; `Fn` + `↑`/`↓` (or `PgUp`/`PgDn` on an extended keyboard) and `/scroll up|down` page the focused Results, Hub, Preview, Help, Settings, command palette, or ActionPicker. Page movement is reducer-only and does not execute actions, refresh modules, or request previews. Mouse reporting remains disabled so terminal-native text selection is preserved |
| Module registry | Available | Manifest + enable/disable; warmup for enabled modules |

## Modules

| Module | Triggers | Status | Default |
| --- | --- | --- | --- |
| Apps | `/app` / `/apps` | Available — fuzzy + session MRU; launch / reveal / copy path | on |
| Calculator | `/calc` / `/calculate` | Available — deterministic arithmetic, units, integer bases, Unix timestamps, and date offsets; strict complete-expression global detection; copy result/equation | on |
| Downloads Inbox | `/dl` / `/downloads` | Available — bounded direct-child recent/large/old/type/text views; open/reveal/copy path; explicit confirmed extension-changing rename and recoverable confirmed Finder Trash | on |
| Packages | `/pkg` / `/packages` / `/brew` | Available — Homebrew-only installed/outdated/formulae/casks/search/info; bounded cancellable direct queries; confirmed exact install/upgrade/uninstall in the interactive terminal; completion names the operation/package and refreshes the active package surface once | on |
| Apple Shortcuts Bridge | `/sc` / `/shortcut` / `/shortcuts` | Available — on-demand list/search/custom-folder views; exact-name View and interactive Run through `/usr/bin/shortcuts`; no warmup enumeration, URL fallback, captured output, or implicit input | on |
| Shell Recall | `/hist` / `/history` | Available — read-only bounded zsh history tail with plain/extended parsing and credential suppression; primary action copies only; command text never enters Recall | on |
| Renewals | `/renew` / `/renewals` | Available — SQLite-backed upcoming/due/30d ledger; integer minor-unit amounts with explicit currency precision; add/edit/paid/cancel/delete/backup; paid advances atomically from the scheduled date with retained month-end/leap anchors; cancel/delete confirm and revalidate identity; hard cap **1000** (updates remain allowed) | on |
| Database Portals | `/db` / `/database` / `/databases` | Available — explicitly provisioned connection launcher; canonical SQLite add/open/reveal plus read-only bounded tables/indexes/DDL and metadata backup; PostgreSQL opens exact `psql` arguments using existing libpq authentication only; production open and metadata-only removal confirm and revalidate; no passwords/DSNs, premature last-open bookkeeping, SQL editor, discovery, or connection tests | **off** (enable in Settings before adding) |
| Screen OCR | `/ocr` | Available — user-selected still-image capture followed by on-device Apple Vision recognition for Simplified Chinese, Traditional Chinese, and English; it never records audio. Recognized plain text is capped at **256 KiB** and pasted without entering Recall or logs; progress says drag/Esc, duplicate execution is rejected, and the module action opens the exact macOS Screen Recording pane; private temporary captures are deleted on every return path | on |
| Windows | `/win` / `/window` / `/windows` | Available — list/search works without Accessibility; unavailable Focus is annotated before execution and `/win` actions can open the exact Accessibility pane. Screen Recording is explained using macOS's “Screen & System Audio Recording” name without implying audio capture. Duplicate genuine Untitled windows are numbered for display only; Hub 1–9 quick focus, prompt digits preserved; default Hub cap **7**, configurable 5–50 | on |
| Git | `/git` | Available — imported projects only; bounded repository discovery; dashboard prioritizes conflict/dirty/ahead; state-aware stage/unstage controls, confirmed tracked-file discard, local log/branches/commit with operation-specific completion feedback and one active-surface refresh. Clean repositories do not offer irrelevant staging actions. No remote, fetch, push, pull, clone, rebase, reset, or `git clean`. | on |
| Runtime | `/run` / `/ports` | Available — on-demand local TCP listener list with project association; copy fields and guarded same-user, identity-rechecked SIGTERM only. No background monitor or SIGKILL. | on |
| Proxy | `/proxy` / `/px` | Available — compact status + group overview; `/proxy group NAME` drills into nodes (on-demand Test delay); mode, `/proxy status` and on-demand `/proxy check`, local macOS HTTP/SOCKS proxy controls, safe Luma Profile import/list/use/delete/refresh, and `/proxy sync` for `LumaNext/proxy.yaml` convention recipes; HTTPS is reported read-only. Clash Verge Profiles are read-only unless Luma-owned. See [Proxy](./PROXY.md). | on |
| Clipboard | `/clip` / `/cb` | Available — history, pin/unpin, `/clip clear`, session `/clip pause [duration]` / `resume` / `status`; concealed/transient password-manager pasteboard types are skipped; paste needs AX; caps: **500** unpinned, **100** pinned; entries over **256 KiB** rejected | on |
| Quicklinks | `/ql` / `/quicklinks` | Available — add/overwrite, open, copy URL, delete, `/ql backup`; hard cap **1000** entries (updates remain allowed at capacity) | on |
| Snippets | `/s` / `/snip` | Available — search/add/overwrite/copy/delete without Accessibility; `/s add-from-clipboard TRIGGER` preserves multiline text; `/s backup`; paste reports AX permission locally; hard cap **1000** entries | on |
| Wordbook | `/wb` / `/wordbook` / `/words` | Available — today/due/new/wrong lists; `/wb review` builds a due-first daily queue and fills it with new words to the remaining goal; specific due/new/wrong review queues remain available. Enter/Space reveal, 1/2/3 grade, m mastered with confirmation, s skip, Esc exit; CSV/clipboard import, daily goal, `/wb backup`. Search/perform honor cancel tokens | on |
| Records | `/rec` / `/record` | Available — SQLite-backed media log; search/browse plus `/rec recent`, `/rec unrated`, `/rec top`; add/rate/note and ActionPicker edit/remove; `/rec backup`; CLI import is dry-run by default and `--apply` is ledger-backed with a LumaNext backup, source Markdown stays read-only | on |
| Projects | `/p` / `/proj` / `/project` | Available — recall-ranked manually imported projects; Enter opens `/proj show PATH`, which aggregates Continue, on-demand Git status, associated Runtime listeners, matching Command Recipes, bounded files, Finder, an available editor CLI, and a project-rooted terminal. `/proj add/import PATH`, `/proj remove NAME\|PATH`, `/proj browse`; canonical existing non-symlink paths, duplicate rejection, config-only removal | on |
| Command Recipes | `/cmd` / `/recipe` / `/recipes` | Available — default surface shows runnable current-directory variants; `/cmd all [filter]` includes inapplicable recipes after runnable rows; `.git/` repositories and `.git` worktrees both match; executable symlinks are followed. `/cmd project PATH` evaluates against an exact imported project; ordered `program + args`; user TOML + built-ins. See [Command Recipes](./COMMAND_RECIPES.md). | on |
| SSH | `/ssh` | Available — reads `~/.ssh/config` Host aliases and automatically refreshes config/Include files on targeted visits; `/ssh fav` / `/ssh recent` / `/ssh rename` / explicit `/ssh reload`; favorite/recent metadata in `ssh_meta.sqlite` with a **1000-row** cap; optional passwords stay in private macOS Keychain accounts and are supplied through OpenSSH AskPass; Enter connects in current terminal; SFTP + copy alias actions. See [SSH](./SSH.md). | on |
| Timers | `/tm` / `/timer` / `/timers` | Available — stopwatch + countdown/Pomodoro; start/pause/resume/reset/delete; running/paused timers appear as live Hub Continue items; state in `timers.sqlite`, hard cap **256**; speech alert on completion while Luma is running (no daemon — graceful quitting pauses running timers). In-process 1s poller cancels on teardown; search/perform honor cancel | on |
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
- **Resolve:** macOS adapter runs `ssh -G <alias>`; entering a targeted `/ssh` surface re-reads
  config/Includes and clears resolved-host cache. `/ssh reload` is the explicit equivalent.
- **Connect:** Opens the in-TUI **SSH Workspace** (embedded PTY). Compat mode still suspends → `ssh <alias>` → resume. Successful exit (`0`) records connection metadata. SFTP keeps the suspend handoff.
- **Queries:** `/ssh `, `/ssh <needle>`, `/ssh fav`, `/ssh recent`, `/ssh reload`, `/ssh rename ALIAS NAME` (case-insensitive `rename` prefix; name may contain spaces).
- **CLI:** `luma ssh list|connect|sftp|favorite|unfavorite|rename`.
- **Search honesty:** missing config → `not_configured`; parse or `ssh` binary errors → `unavailable`. Preview never shows private key contents.
- **Details:** [SSH.md](./SSH.md).

### Continue and global recall

- Global search contributors are Apps, Calculator (strict complete expressions only), Windows,
  Projects, Command Recipes, SSH, Clipboard, Snippets, Quicklinks, and Git.
  Informational/unavailable/no-match rows are excluded. Results are
  capped at 12 per module and 60 total and remain relevance-first; after two rows from one module,
  a near-equivalent alternative (within three score points) may diversify the page. Recall boosts
  are deliberately smaller than a semantic match band. Records and Wordbook remain
  targeted-only because their dense historical rows lack a clear bounded global-search benefit.
- A successful natural primary action records only bounded recall metadata in `recall.sqlite`:
  object/module/kind, natural primary action, a safe display title, optional project association,
  count, and last use. Successful secondary actions do not create usage entries; successful
  destructive actions evict the object's entry. Failed/cancelled actions are not recorded.
  Clipboard bodies, snippet bodies, SSH configuration, proxy endpoints, Calculator expressions,
  Screen OCR text, and search text are never copied into Recall.
- The empty Hub renders at most three live-or-compatible Continue rows after Windows. Running or
  paused Timers are projected first; remaining slots come from Recall only after the owning module
  re-reads the current object and restores its real payload, risk, and confirmation requirement.
  Missing objects are pruned; temporary store failures retain metadata but skip the row. Modules
  without a safe rehydration contract (including direct Git and Runtime objects) remain
  recall-ranked in global search but are not Hub-continued. `/proj show PATH` may offer one
  project-scoped Continue row by converting stored metadata back into a slash surface and letting
  the destination module re-read live state.

### Project Workbench

- `/proj` lists only explicit imports and adds a bounded Recall score by `project_path`; no project
  visit log or source-module data is duplicated in Projects.
- `/proj show NAME|PATH` resolves an exact imported project. Ambiguous names require the full path.
  Rows link through generic `OpenSurface` outcomes to `/git repo PATH`, `/run PATH`,
  `/cmd project PATH`, and `/proj browse PATH`; central TUI routing has no Projects special-case.
- Git, Runtime, Recipes, and Recall are read on demand through their existing ports/repositories.
  Failures remain visible on the corresponding row and Projects never becomes a central doctor.
- Open terminal validates the imported path again, suspends the TUI, and starts `/bin/zsh` with the
  project path as a positional argument (not interpolated shell text). Removing an import still
  changes settings only and never deletes the directory.
- Open editor is offered only when `code`, `cursor`, `zed`, `nvim`, or `vim` is available on the
  process PATH; the chosen CLI receives the validated project path as one direct argument.

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
- **Queries:** `/tm ` lists timers; `/tm pomo|pomodoro|cd|countdown [minutes] [name]`,
  `/tm 25`, and `/tm sw|start|stopwatch [name]` create+start rows.
- **Actions:** Start / Pause / Resume / Reset; Delete (confirm).
- **Hub:** running and paused timers occupy the bounded Continue section before recalled objects;
  Enter performs the current natural action (Pause/Resume) against live state.
- **Alerts:** speech (“… done”) when a countdown finishes **while Luma is running**. Quitting pauses running timers so elapsed time does not advance silently offline.
- **Concurrency / cancel:** warmup starts a session-scoped 1s poller; teardown cancels it and bumps a generation so in-flight ticks cannot alert after shutdown. Search and perform return early when their cancel token fires.
- **Honesty:** store/clock failures surface as `unavailable` rows.

### Wordbook (concurrency)

- Search and perform check cancel before mutating or speaking.
- Import / pasteboard / speech paths use cancel-aware awaits so Esc / superseded ops do not leave half-applied UI side effects.
- Review queue load is engine-owned (`LoadWordbookReview`); grading still goes through normal ExecuteAction cancel.
- `/wb review` means the daily queue: all due words first, then new words until the remaining
  daily goal is filled. `/wb review due|new|wrong` keeps queue-specific control.

### Clipboard capacity

Aligned with `luma-storage` clipboard store constants:

- **500** unpinned history rows (soft cap; pinned rows are never evicted by this cap).
- **100** pinned rows (hard cap; unpin one before pinning another; pinned data is never silently
  deleted).
- **256 KiB** max bytes per entry (`MAX_ENTRY_BYTES`); larger pastes are rejected.
- AppKit concealed/transient/autogenerated pasteboard types (including common password-manager
  markers) are ignored by history capture. Direct user-requested paste still reads text.
- `/clip pause` lasts for the current session; an optional bounded duration resumes automatically.
  `/clip status` reports the current capture state.

### Backups and logs

- `/wb backup`, `/rec backup`, `/renew backup`, `/db backup`, `/s backup`, and `/ql backup` use SQLite `VACUUM INTO` plus an
  atomic rename under `~/Library/Application Support/LumaNext/backups/`.
- `~/Library/Logs/LumaNext/luma.log` rotates at 5 MiB and retains `luma.log.1` through
  `luma.log.3`.

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
