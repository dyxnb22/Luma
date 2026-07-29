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
| Apps, Calculator, Downloads Inbox, Packages, Apple Shortcuts Bridge, Shell Recall, Renewals, Database Portals, Screen OCR, Windows, Git, Runtime, Proxy, Clipboard, Quicklinks, Snippets, Wordbook, Projects, Records, Command Recipes, SSH, Timers, Secrets | Window layouts, Menu search, Browser tabs, signed-host Translate |
| Honest permission / unavailable states in each surface | Release soak, deny-as-policy, marketing docs |
| Global Command+Space activation of the one TUI session | A native product UI: search box, results, sidebar, settings, or overlays in Swift |
| Module-local status and remediation rows | Centralized `doctor` command or diagnostics overlay |

## Docs

| Doc | Path |
| --- | --- |
| Operator guide | [`rust/README.md`](rust/README.md) |
| Module status | [`rust/docs/MODULES.md`](rust/docs/MODULES.md) |
| Codebase governance | [`rust/docs/GOVERNANCE.md`](rust/docs/GOVERNANCE.md) |
| Implemented module contract | [`rust/docs/SELECTED_MODULES_PLAN.md`](rust/docs/SELECTED_MODULES_PLAN.md) |
| Archived eight-module handoff prompt | [`rust/docs/CODEX_TERRA_SELECTED_MODULES_PROMPT.md`](rust/docs/CODEX_TERRA_SELECTED_MODULES_PROMPT.md) |
| SSH Connections | [`rust/docs/SSH.md`](rust/docs/SSH.md) |
| Proxy (Mihomo) | [`rust/docs/PROXY.md`](rust/docs/PROXY.md) |
| macOS smoke checks | [`rust/docs/MACOS_SMOKE.md`](rust/docs/MACOS_SMOKE.md) |
| Personal usage log | [`rust/docs/USAGE_LOG_TEMPLATE.md`](rust/docs/USAGE_LOG_TEMPLATE.md) |
| Empty Hub and keyboard behavior | [`rust/docs/hub.md`](rust/docs/hub.md) |
| Decisions | [`rust/docs/adr/`](rust/docs/adr/) |

## Data

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active app support (settings, `ssh_meta.sqlite`, `recall.sqlite`, `renewals.sqlite`, `database_portals.sqlite`, other stores) |
| `~/Library/Logs/LumaNext/` | Runtime logs (5 MiB rotation, three archives) |
| `~/.ssh/config` | OpenSSH Host aliases (read-only for `luma.ssh`) |
| macOS Keychain | Optional local SSH passwords and other private Luma references; values do not enter LumaNext files |

The empty Hub lists visible windows, up to three live-or-privacy-safe Continue items, and modules. Press `1`–`9`
to focus a numbered window; Continue and module rows are never digit targets. Digits remain
ordinary prompt input in search fields. Interactive commands require a leading
`/`: `/win`, `/wb review [today|due|new|wrong]`, `/rec`, and `/ssh `; unprefixed input is global search.
`Ctrl-/` opens the command palette, whose module subcommands, parameter placeholders, and examples
come from the same manifest descriptions used by `/help` and slash-command completion.
`Fn` + `↑`/`↓` (or `PgUp`/`PgDn` on an extended keyboard) and the palette actions `/scroll up` and `/scroll down` page the focused Results,
Hub, Preview, Help, Settings, command palette, or ActionPicker without running a business action.
The Rust TUI uses layered panels, a single focused accent, full-row selection, and a contextual
shortcut footer; the native host does not own or duplicate this visual system.
`/proj` lists manually imported projects; Enter opens a single-project workbench that brings
together recent activity, local Git state, associated listeners, Command Recipes, file browsing,
an available editor CLI, and a project terminal without moving those source concerns into the
Projects module.
Notes are handled by external tools such as Obsidian; Luma does not index or modify note vaults.
