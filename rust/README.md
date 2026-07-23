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

## Native menu-bar companion

Build the companion into a stable app path before granting macOS permissions or enabling
Launch at Login. The script performs a local ad-hoc signature with the stable bundle
identifier `com.luma.next.menubar` and verifies the completed bundle:

```bash
cd rust
bash scripts/build_menubar_app.sh "$HOME/Applications/Luma Menu Bar.app"
open "$HOME/Applications/Luma Menu Bar.app"
```

Run the same command to update the app in place. Keep the app at that path so its bundle and
TCC identity remain stable; do not launch the temporary copy under `target/` after authorizing
the installed copy. Set `CODESIGN_IDENTITY` if a local signing certificate is preferred.

The menu-bar process has its own macOS permissions. Grant Accessibility to `Luma Menu Bar.app`
for window focus, and Screen Recording if full window titles are needed. The TUI/Terminal
permission entry is separate. Launch at Login requires the bundled app to be installed at a
stable path and may require approval in System Settings.

Optional local hygiene: `bash scripts/check_architecture.sh`.

**Fixtures:** `fixtures/notes-workspaces/` for Notes scan/index tests; `fixtures/legacy/` for migrate blackbox.

See [`docs/MODULES.md`](docs/MODULES.md) for module status.
See [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) for personal codebase governance (inventory sync, soft file limits, anti-patterns).
See [`docs/COMMAND_RECIPES.md`](docs/COMMAND_RECIPES.md) for command templates, TOML config, and safety.
See [`docs/SSH.md`](docs/SSH.md) for SSH Connections (`~/.ssh/config` launcher, metadata, CLI).
See [`docs/PROXY.md`](docs/PROXY.md) for Mihomo/Clash Verge Profile behavior, safety boundaries,
supported subscription formats, and rollback semantics.
See [`docs/MACOS_SMOKE.md`](docs/MACOS_SMOKE.md) for real macOS permission, menu-bar, terminal,
window, Keychain, clipboard, and proxy smoke checks.
See [`docs/USAGE_LOG_TEMPLATE.md`](docs/USAGE_LOG_TEMPLATE.md) for an optional privacy-preserving
14-day local usage experiment.

## Data roots

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active settings / stores (`ssh_meta.sqlite`, clipboard, records, …) |
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

- Empty Hub: `1`–`9` focuses visible window rows; status, “more”, and module rows are not numbered.
- `/win`: `1`–`9` works only while the result list is focused. Digits typed in the prompt are never hijacked.
- `/wb due`, `/wb new`, `/wb wrong`: normal lists. `/wb review due|new|wrong`: Enter/Space reveals, `1/2/3` grades, `m` masters after confirmation, `s` skips, Esc exits. `/wb import PATH` accepts a regular non-symlink UTF-8 CSV up to 512 KiB.
- `/rec`: searches Records. Use `/rec browse`, `/rec add CATEGORY NAME | rating | note`, `/rec rate ID SCORE`, and `/rec note ID TEXT`.
- `/proj`: plain search shows only manually imported projects. Use `/proj add/import PATH`, `/proj remove NAME|PATH`, and `/proj browse`.
- `/ssh`: lists Host aliases from `~/.ssh/config`; Enter runs `ssh <alias>` in the current terminal (TUI suspends first); `/ssh fav` / `/ssh recent` / `/ssh rename ALIAS NAME` / `/ssh reload`; action picker: Open SFTP, Copy alias, Favorite/Unfavorite, Delete local metadata. See [`docs/SSH.md`](docs/SSH.md).
- There is no `luma doctor`, `:doctor`, or diagnostics overlay. Modules report `permission`, `unavailable`, or `not_configured` locally when applicable.

Optional importers: `luma migrate …` with an explicit legacy path (dry-run by default).
