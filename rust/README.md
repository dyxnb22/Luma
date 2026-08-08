# Luma (Rust CLI/TUI) — personal use

Solo daily driver. **No public-release checklist.** Prefer fixing what you hit while using it.

## Commands

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox

cargo run -p luma -- query "/app safari" --json
cargo run -p luma -- query "/clip " --json
cargo run -p luma -- query "/win " --json
cargo run -p luma -- query "/cmd test" --json
cargo run -p luma -- query "/git" --json
cargo run -p luma -- query "/run" --json
cargo run -p luma -- query "/proxy status" --json
cargo run -p luma -- query "/proxy check" --json
cargo run -p luma -- query "/calc 128 MiB in GiB" --json
cargo run -p luma -- query "/dl recent" --json
cargo run -p luma -- query "/pkg outdated" --json
cargo run -p luma -- query "/sc folders" --json
cargo run -p luma -- query "/hist rg" --json
cargo run -p luma -- query "/renew 30d" --json
cargo run -p luma -- query "/db" --json       # default-off until enabled in Settings
cargo run -p luma -- query "/ocr" --json
cargo run -p luma -- cmd list --json
cargo run -p luma -- cmd show git-status --json
printf '%s' 'secret' | cargo run -p luma -- secrets set my-label
cargo run -p luma -- modules list --json
cargo run -p luma -- config get --json
cargo run -p luma -- config set --records-root ~/Documents/Notes/Records
cargo run -p luma -- config set --projects-root ~/dev
cargo run -p luma -- config set --import-project ~/dev/myapp
cargo run -p luma -- config set --remove-project myapp
cargo run -p luma -- record import --root ~/Documents/Notes/Records       # dry-run
cargo run -p luma -- record import --root ~/Documents/Notes/Records --apply
cargo run -p luma -- record browse
cargo run -p luma -- record browse --category 电影
cargo run -p luma -- record import-status
cargo run -p luma -- record backup
cargo run -p luma -- record rate 1 9
cargo run -p luma -- record note 1 '值得重看'
cargo run -p luma -- record remove 1 --yes
cargo run -p luma   # interactive TUI
```

## Native workbench host

`Luma.app` is a thin Swift/AppKit window that hosts the same `luma tui` process in a real PTY
behind a global activation hotkey ([ADR-0007](docs/adr/0007-native-workbench-host.md)). It owns
the window, the PTY, activation, and lifecycle — never product UI or module data.

```bash
cd rust
swift test --package-path native/luma-workbench
swift build --package-path native/luma-workbench -c release
bash scripts/build_workbench_app.sh "$HOME/Applications/Luma.app"
open "$HOME/Applications/Luma.app"
```

Re-run the build script to update in place and keep the app at that path so Finder, Login Items,
and the bundle identifier (`com.luma.next.workbench`) remain consistent. The default ad-hoc
signature is deliberately convenient for local builds, but it does **not** guarantee that macOS
TCC grants survive a rebuild: re-check module permissions after updating. Set
`CODESIGN_IDENTITY` to a stable local certificate when you need TCC identity continuity.

Behavior worth knowing:

- On a fresh install, Option+Space shows the window and focuses the terminal; pressing the saved
  shortcut again while Luma is active hides the window and reactivates the app you came from.
  If another app owns it, Luma offers explicit alternatives and remembers the user's choice; it
  never silently changes an existing shortcut. Command+Space remains available but is offered
  last because macOS normally reserves it for Spotlight.
- The close button hides the window. The TUI keeps running; Cmd+Q (or Quit Luma) terminates it.
- If the TUI exits, the next activation starts a fresh session.
- The host is an accessory app, so it has no Dock icon and no visible menu bar. Cmd+C, Cmd+V,
  Cmd+A, Cmd+W and Cmd+Q still work; there is no Launch at Login toggle. To start it at login, add
  `$HOME/Applications/Luma.app` under System Settings → General → Login Items.
- The hotkey uses Carbon `RegisterEventHotKey` and needs no Accessibility permission. Modules that
  need Accessibility (for example `/win` focus) prompt under the `Luma.app` identity, separately
  from a copy run directly in Terminal.
- The host samples combined host/TUI RSS every 30 seconds and records the session peak in the local
  unified log. On macOS memory pressure it reduces terminal scrollback; no metrics leave the Mac.
- Cmd+Q sends SIGTERM to the PTY process group. The TUI handles it as a graceful shutdown so module
  teardown runs; the host uses SIGKILL only if the group has not exited after three seconds.

Optional local hygiene: `bash scripts/check_architecture.sh`.

**Fixtures:** `fixtures/legacy/` for migrate blackbox.

See [`docs/MODULES.md`](docs/MODULES.md) for module status.
See [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) for personal codebase governance (inventory sync, soft file limits, anti-patterns).
See [`docs/COMMAND_RECIPES.md`](docs/COMMAND_RECIPES.md) for command templates, TOML config, and safety.
See [`docs/PROXY.md`](docs/PROXY.md) for Mihomo/Clash Verge Profile behavior, safety boundaries,
supported subscription formats, and rollback semantics.
See [`docs/MACOS_SMOKE.md`](docs/MACOS_SMOKE.md) for real macOS permission, local utility modules,
workbench host, terminal, window, Keychain, clipboard, and proxy smoke checks.
See [`docs/USAGE_LOG_TEMPLATE.md`](docs/USAGE_LOG_TEMPLATE.md) for an optional privacy-preserving
14-day local usage experiment.

## Data roots

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active settings / stores (`recall.sqlite`, `renewals.sqlite`, `database_portals.sqlite`, clipboard, records, …) |
| `~/Library/Application Support/LumaNext/command-recipes.toml` | User command recipe definitions |
| `~/Library/Application Support/LumaNext/command-recipes-meta.sqlite` | Recipe favorites / usage metadata |
| `~/Library/Logs/LumaNext/` | Logs (`luma.log`, rotated at 5 MiB with three archives) |

Tests must use tempfile + `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`.
Legacy `ssh_meta.sqlite` and SSH-password Keychain entries from older builds are left untouched;
current Luma builds neither open nor mutate them.

`luma record import` is dry-run by default; `--apply` writes the Records database, its LumaNext
backup, and migration ledger, never the Markdown source files.

## TUI quick reference

- The visual shell uses a layered dark/light semantic palette, one accent border for the focused
  pane, full-width selection bands, and a one-line contextual shortcut bar. `COLORFGBG` selects
  the light palette in auto mode; `LUMA_TUI_ASCII=1` keeps simpler decorative glyphs and
  non-rounded panel chrome.
- Commands use a leading `/`, for example `/proj`, `/rec browse`, `/cmd test`, `/settings`,
  and `/help`. Input without `/` is always treated as a global search.
- `Ctrl-/` opens the searchable command palette; `/commands [filter]` opens the same surface.
  Enabled modules publish their real subcommands, argument placeholders, and short examples there
  in task groups and in `/help`; all rows still come from module `CommandSpec` metadata. Partial
  slash commands show bounded completion candidates.
- `⌥↑` / `⌥↓` page the focused surface. Compact-Mac `fn↑` / `fn↓` is a compatibility
  alias; no dedicated Page keys are required. `/scroll up` and `/scroll down` expose the same pure
  navigation through the command palette, including Results, the empty Hub, Preview, Help,
  Settings, the palette itself, and ActionPicker. Paging never runs an action or requests module I/O.

- Configure daily-use values without leaving the TUI: `/settings projects-root PATH`,
  `/settings import-project PATH`, `/settings records-root PATH|none`,
  `/settings clipboard-retention-days N`, `/settings secrets-idle-lock-secs N`, and
  `/settings hub-windows-max N`.

- Empty Hub: `1`–`9` focuses visible window rows; status, “more”, and module rows are not numbered.
- Empty Hub Continue: up to three items appear after Windows. Live running/paused timers come
  first, followed by privacy-safe recent objects revalidated against their owning modules; stale
  objects are pruned and each row uses its current natural primary action, risk, and confirmation.
  They are not window rows and never receive a digit shortcut. Recall is bounded to 1,000 metadata rows and
  never stores clipboard bodies, snippet bodies, or submitted search text.
- `/win`: `1`–`9` works only while the result list is focused. Digits typed in the prompt are never hijacked.
- `/wb today`, `/wb due`, `/wb new`, `/wb wrong`: normal lists. `/wb review` starts today's
  queue (due first, then new words up to the remaining goal); append `due|new|wrong` for a specific
  queue. Enter/Space reveals, `1/2/3` grades, `m` masters after confirmation, `s` skips, Esc exits.
  `/wb import PATH` accepts a regular non-symlink UTF-8 CSV up to 512 KiB; `/wb backup` writes an
  atomic SQLite snapshot under `LumaNext/backups/`.
- `/rec`: searches Records. Use `/rec recent|unrated|top`, `/rec browse`,
  `/rec add CATEGORY NAME | rating | note`, `/rec rate ID SCORE`, `/rec note ID TEXT`, and
  `/rec backup`.
- `/clip pause [30s|10m|2h|1d]`, `/clip resume`, and `/clip status` control session capture.
  Concealed/transient password-manager pasteboard types are never stored in history.
- `/calc` evaluates deterministic arithmetic, units, integer bases, Unix timestamps, and date
  offsets. Only strict complete expressions may appear in global search.
- `/dl` scans direct children of Downloads only. Rename is explicit and Finder Trash remains
  recoverable; there is no recursive or automatic cleanup.
- `/pkg` is Homebrew-only; `/sc` delegates to Apple Shortcuts; `/hist` reads a bounded zsh-history
  tail. Package mutations confirm, Shortcuts run interactively without implicit input/output
  capture, and Shell Recall can copy but never execute a command.
- `/renew` owns the local recurring-payment ledger. `/db` is default-off and stores only
  non-secret portal metadata; PostgreSQL authentication stays with libpq/`psql`.
- `/ocr` captures one user-selected region and recognizes it locally with Apple Vision. It stores
  no screenshot or OCR history and keeps permission/cancellation states inside the module.
- `/s add-from-clipboard TRIGGER` preserves multiline snippet bodies. `/s backup` and
  `/ql backup` create SQLite snapshots under `LumaNext/backups/`.
- `/proj`: lists only manually imported projects, recall-ranked by recent/frequent associated
  actions. Enter opens `/proj show PATH`, a single-project workbench with Continue, local Git
  summary, associated Runtime listeners, matching Command Recipes, bounded file browsing, Finder,
  the first available editor CLI (`code`, `cursor`, `zed`, `nvim`, `vim`), and an interactive zsh
  rooted at the project. Use `/proj add/import PATH`,
  `/proj remove NAME|PATH`, and `/proj browse`; the workbench reads source modules on demand and
  owns none of their data.
- `/git`: scans only manually imported project roots (bounded depth/count) and prioritizes
  conflict/dirty/ahead repositories. Enter opens `/git repo PATH`; files support stage/unstage,
  a bounded diff preview, and confirmed discard of unstaged tracked-file edits while preserving
  the index (untracked files are never cleaned).
  With the list focused: `s` stage/unstage, `a` stage all, `c` seed a commit message, `b` branches,
  `l` local log, `r` refresh, `d` confirmed discard. `a` stages or unstages all according to the
  current file/control state; branch switches refuse dirty repositories.
- `/run` (or `/ports`): lists local TCP listeners with port, address, PID, process, owner, cwd, and
  imported-project association. Enter opens that project's `/proj show PATH` workbench. SIGTERM is confirmation-gated,
  same-user only, identity-rechecked, and never escalates to SIGKILL.
- `/proxy` is a compact status + group-summary view; `/proxy group NAME` expands nodes.
  `/proxy status` is a read-only snapshot of HTTP/HTTPS/SOCKS settings, controller, and Luma-owned
  profile state. `/proxy check` performs on-demand local route/DNS/loopback/controller checks; it
  has no daemon or probe-port subsystem.
- There is no `luma doctor`, `:doctor`, or diagnostics overlay. Modules report `permission`, `unavailable`, or `not_configured` locally when applicable.

Optional importers: `luma migrate …` with an explicit legacy path (dry-run by default).

Notes are handled by external tools such as Obsidian. Luma does not index, watch, create, or
modify Markdown vaults.
