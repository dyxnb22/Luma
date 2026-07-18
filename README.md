# Luma

Keyboard-first **personal** launcher (Rust CLI/TUI with an optional native menu-bar companion).
Built for solo daily use — **not** for public release or distribution.

```bash
cd rust
cargo run -p luma                 # interactive TUI
cargo run -p luma -- query "/app " --json
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```

## Scope

| In | Out |
| --- | --- |
| Local TUI + CLI plus glance-only native menu bar on your Mac | App Store / notarized shipping |
| Apps, Windows, Proxy, Clipboard, Notes, Quicklinks, Snippets, Wordbook, Projects, Records, Command Recipes, SSH, Timers, Secrets | Window layouts, Menu search, Browser tabs, signed-host Translate |
| Honest permission / unavailable states in each surface | Release soak, deny-as-policy, marketing docs |
| Module-local status and remediation rows | Centralized `doctor` command or diagnostics overlay |

## Docs

| Doc | Path |
| --- | --- |
| Operator guide | [`rust/README.md`](rust/README.md) |
| Module status | [`rust/docs/MODULES.md`](rust/docs/MODULES.md) |
| Codebase governance | [`rust/docs/GOVERNANCE.md`](rust/docs/GOVERNANCE.md) |
| SSH Connections | [`rust/docs/SSH.md`](rust/docs/SSH.md) |
| Proxy (Mihomo) | [`rust/docs/PROXY.md`](rust/docs/PROXY.md) |
| Empty Hub and keyboard behavior | [`rust/docs/hub.md`](rust/docs/hub.md) |
| Decisions | [`rust/docs/adr/`](rust/docs/adr/) |

## Data

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active app support (settings, `ssh_meta.sqlite`, stores) |
| `~/Library/Logs/LumaNext/` | Runtime logs |
| `~/.ssh/config` | OpenSSH Host aliases (read-only for `luma.ssh`) |

The empty Hub lists visible windows and modules. Press `1`–`9` to focus a numbered window;
digits remain ordinary prompt input in search fields. Interactive commands require a leading
`/`: `/win`, `/wb review due|new|wrong`, `/rec`, and `/ssh `; unprefixed input is global search.
