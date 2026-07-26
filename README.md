# Luma

Keyboard-first **personal** launcher: a Rust CLI/TUI with an optional thin native window that
hosts the same TUI behind a global hotkey.
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
| Local TUI + CLI and a thin native PTY host window on your Mac | App Store / notarized shipping |
| Apps, Windows, Git, Runtime, Proxy, Clipboard, Quicklinks, Snippets, Wordbook, Projects, Records, Command Recipes, SSH, Timers, Secrets | Window layouts, Menu search, Browser tabs, signed-host Translate |
| Honest permission / unavailable states in each surface | Release soak, deny-as-policy, marketing docs |
| Global Option+Space activation of the one TUI session | A native product UI: search box, results, sidebar, settings, or overlays in Swift |
| Module-local status and remediation rows | Centralized `doctor` command or diagnostics overlay |

## Docs

| Doc | Path |
| --- | --- |
| Operator guide | [`rust/README.md`](rust/README.md) |
| Module status | [`rust/docs/MODULES.md`](rust/docs/MODULES.md) |
| Codebase governance | [`rust/docs/GOVERNANCE.md`](rust/docs/GOVERNANCE.md) |
| SSH Connections | [`rust/docs/SSH.md`](rust/docs/SSH.md) |
| Proxy (Mihomo) | [`rust/docs/PROXY.md`](rust/docs/PROXY.md) |
| macOS smoke checks | [`rust/docs/MACOS_SMOKE.md`](rust/docs/MACOS_SMOKE.md) |
| Personal usage log | [`rust/docs/USAGE_LOG_TEMPLATE.md`](rust/docs/USAGE_LOG_TEMPLATE.md) |
| Empty Hub and keyboard behavior | [`rust/docs/hub.md`](rust/docs/hub.md) |
| Decisions | [`rust/docs/adr/`](rust/docs/adr/) |

## Data

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active app support (settings, `ssh_meta.sqlite`, `recall.sqlite`, stores) |
| `~/Library/Logs/LumaNext/` | Runtime logs |
| `~/.ssh/config` | OpenSSH Host aliases (read-only for `luma.ssh`) |

The empty Hub lists visible windows, a privacy-safe Continue section, and modules. Press `1`–`9`
to focus a numbered window; Continue and module rows are never digit targets. Digits remain
ordinary prompt input in search fields. Interactive commands require a leading
`/`: `/win`, `/wb review due|new|wrong`, `/rec`, and `/ssh `; unprefixed input is global search.
`/proj` lists manually imported projects; Enter opens a single-project workbench that brings
together recent activity, local Git state, associated listeners, Command Recipes, file browsing,
an available editor CLI, and a project terminal without moving those source concerns into the
Projects module.
Notes are handled by external tools such as Obsidian; Luma does not index or modify note vaults.
