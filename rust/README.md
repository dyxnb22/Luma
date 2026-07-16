# Luma (Rust CLI/TUI) — personal use

Solo daily driver. **No public-release checklist.** Prefer fixing what you hit while using it.

## Commands

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox

cargo run -p luma -- query "app safari" --json
cargo run -p luma -- query "clip" --json   # bare trigger OK in CLI (targeted clip)
cargo run -p luma -- query "n" --json      # bare trigger OK in CLI (targeted notes)
cargo run -p luma -- query "win" --json
cargo run -p luma -- query "ssh production" --json
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

Optional local hygiene: `bash scripts/check_architecture.sh`.

**Fixtures:** `fixtures/notes-workspaces/` for Notes scan/index tests; `fixtures/legacy/` for migrate blackbox.

See [`docs/MODULES.md`](docs/MODULES.md) for module status.
See [`docs/SSH.md`](docs/SSH.md) for SSH Connections (`~/.ssh/config` launcher, metadata, CLI).
See [`docs/PROXY.md`](docs/PROXY.md) for Mihomo/Clash Verge Profile behavior, safety boundaries,
supported subscription formats, and rollback semantics.

## Data roots

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active settings / stores (`ssh_meta.sqlite`, clipboard, records, …) |
| `~/Library/Logs/LumaNext/` | Logs |
| `~/.ssh/config` | OpenSSH Host aliases — read by `luma.ssh` only (not modified) |

Tests must use tempfile + `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`.

`luma record import` is dry-run by default; `--apply` writes the Records database, its LumaNext
backup, and migration ledger, never the Markdown source files.

## TUI quick reference

- Empty Hub: `1`–`9` focuses visible window rows; status, “more”, and module rows are not numbered.
- `win`: `1`–`9` works only while the result list is focused. Digits typed in the prompt are never hijacked.
- `wb due`, `wb new`, `wb wrong`: normal lists. `wb review due|new|wrong`: Enter/Space reveals, `1/2/3` grades, `m` masters after confirmation, `s` skips, Esc exits. `wb import PATH` accepts a regular non-symlink UTF-8 CSV up to 512 KiB.
- `rec`: searches Records. Use `rec browse`, `rec add CATEGORY NAME | rating | note`, `rec rate ID SCORE`, and `rec note ID TEXT`.
- `proj`: plain search shows only manually imported projects. Use `proj add/import PATH`, `proj remove NAME|PATH`, and `proj browse`.
- `ssh`: lists Host aliases from `~/.ssh/config`; Enter runs `ssh <alias>` in the current terminal (TUI suspends first); `ssh fav` / `ssh recent` / `ssh rename ALIAS NAME` / `ssh reload`; action picker: Open SFTP, Copy alias, Favorite/Unfavorite, Delete local metadata. See [`docs/SSH.md`](docs/SSH.md).
- There is no `luma doctor`, `:doctor`, or diagnostics overlay. Modules report `permission`, `unavailable`, or `not_configured` locally when applicable.

Optional importers: `luma migrate …` with an explicit legacy path (dry-run by default).
