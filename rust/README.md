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
cargo run -p luma -- query "/n " --json
cargo run -p luma -- query "/win " --json
cargo run -p luma -- query "/cmd test" --json
cargo run -p luma -- query "/git" --json
cargo run -p luma -- query "/run" --json
cargo run -p luma -- query "/proxy status" --json
cargo run -p luma -- query "/proxy check" --json
cargo run -p luma -- cmd list --json
cargo run -p luma -- cmd show git-status --json
cargo run -p luma -- query "/ssh production" --json
cargo run -p luma -- ssh list --json
cargo run -p luma -- ssh connect production
cargo run -p luma -- ssh sftp production
cargo run -p luma -- ssh favorite production
cargo run -p luma -- ssh unfavorite production
cargo run -p luma -- ssh rename production "Prod server"
printf '%s' 'secret' | cargo run -p luma -- secrets set my-label
cargo run -p luma -- modules list --json
cargo run -p luma -- config get --json
cargo run -p luma -- config set --notes-root /path/to/notes
cargo run -p luma -- config set --records-root ~/Documents/Notes/Records
cargo run -p luma -- config set --projects-root ~/dev
cargo run -p luma -- config set --import-project ~/dev/myapp
cargo run -p luma -- config set --remove-project myapp
cargo run -p luma -- config set --notes-exclude 'private/*'
cargo run -p luma -- config set --clear-notes-excludes
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
behind a global Option+Space hotkey ([ADR-0007](docs/adr/0007-native-workbench-host.md)). It owns
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

- Option+Space shows the window and focuses the terminal; pressing it again while Luma is active
  hides the window and reactivates the app you came from.
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

**Fixtures:** `fixtures/notes-workspaces/` for Notes scan/index tests; `fixtures/legacy/` for migrate blackbox.

See [`docs/MODULES.md`](docs/MODULES.md) for module status.
See [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) for personal codebase governance (inventory sync, soft file limits, anti-patterns).
See [`docs/COMMAND_RECIPES.md`](docs/COMMAND_RECIPES.md) for command templates, TOML config, and safety.
See [`docs/SSH.md`](docs/SSH.md) for SSH Connections (`~/.ssh/config` launcher, metadata, CLI).
See [`docs/PROXY.md`](docs/PROXY.md) for Mihomo/Clash Verge Profile behavior, safety boundaries,
supported subscription formats, and rollback semantics.
See [`docs/MACOS_SMOKE.md`](docs/MACOS_SMOKE.md) for real macOS permission, workbench host,
terminal, window, Keychain, clipboard, and proxy smoke checks.
See [`docs/USAGE_LOG_TEMPLATE.md`](docs/USAGE_LOG_TEMPLATE.md) for an optional privacy-preserving
14-day local usage experiment.

## Data roots

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active settings / stores (`ssh_meta.sqlite`, `recall.sqlite`, clipboard, records, …) |
| `~/Library/Application Support/LumaNext/command-recipes.toml` | User command recipe definitions |
| `~/Library/Application Support/LumaNext/command-recipes-meta.sqlite` | Recipe favorites / usage metadata |
| `~/Library/Logs/LumaNext/` | Logs |
| `~/.ssh/config` | OpenSSH Host aliases — read by `luma.ssh` only (not modified) |

Tests must use tempfile + `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`.

`luma record import` is dry-run by default; `--apply` writes the Records database, its LumaNext
backup, and migration ledger, never the Markdown source files.

## TUI quick reference

- Commands use a leading `/`, for example `/ssh prod`, `/rec browse`, `/cmd test`, `/settings`,
  and `/help`. Input without `/` is always treated as a global search.

- Set up a notes or projects workspace without leaving the TUI: `/settings notes-root PATH`,
  `/settings projects-root PATH`, or `/settings import-project PATH`.

- Empty Hub: `1`–`9` focuses visible window rows; status, “more”, and module rows are not numbered.
- Empty Hub Continue: up to five privacy-safe recent objects appear after Windows. They use their
  natural primary action; they are not window rows and never receive a digit shortcut. Recall is
  bounded to 1,000 metadata rows and never stores clipboard bodies, snippet bodies, SSH config,
  or submitted search text.
- `/win`: `1`–`9` works only while the result list is focused. Digits typed in the prompt are never hijacked.
- `/wb due`, `/wb new`, `/wb wrong`: normal lists. `/wb review due|new|wrong`: Enter/Space reveals, `1/2/3` grades, `m` masters after confirmation, `s` skips, Esc exits. `/wb import PATH` accepts a regular non-symlink UTF-8 CSV up to 512 KiB.
- `/rec`: searches Records. Use `/rec browse`, `/rec add CATEGORY NAME | rating | note`, `/rec rate ID SCORE`, and `/rec note ID TEXT`.
- `/proj`: plain search shows only manually imported projects. Use `/proj add/import PATH`, `/proj remove NAME|PATH`, and `/proj browse`.
- `/git`: scans only manually imported project roots (bounded depth/count) and prioritizes
  conflict/dirty/ahead repositories. Enter opens `/git repo PATH`; files support stage/unstage,
  a bounded diff preview, and confirmed discard of unstaged tracked-file edits while preserving
  the index (untracked files are never cleaned).
  With the list focused: `s` stage/unstage, `a` stage all, `c` seed a commit message, `b` branches,
  `l` local log, `r` refresh, `d` confirmed discard. `a` stages or unstages all according to the
  current file/control state; branch switches refuse dirty repositories.
- `/run` (or `/ports`): lists local TCP listeners with port, address, PID, process, owner, cwd, and
  imported-project association. Enter opens an associated project. SIGTERM is confirmation-gated,
  same-user only, identity-rechecked, and never escalates to SIGKILL.
- `/proxy status` is a read-only snapshot of HTTP/HTTPS/SOCKS settings, controller, and Luma-owned
  profile state. `/proxy check` performs on-demand local route/DNS/loopback/controller checks; it
  has no daemon or probe-port subsystem.
- `/ssh`: lists Host aliases from `~/.ssh/config`; Enter runs `ssh <alias>` in the current terminal (TUI suspends first); `/ssh fav` / `/ssh recent` / `/ssh rename ALIAS NAME` / `/ssh reload`; action picker: Open SFTP, Copy alias, Favorite/Unfavorite, Delete local metadata. See [`docs/SSH.md`](docs/SSH.md).
- There is no `luma doctor`, `:doctor`, or diagnostics overlay. Modules report `permission`, `unavailable`, or `not_configured` locally when applicable.

Optional importers: `luma migrate …` with an explicit legacy path (dry-run by default).
