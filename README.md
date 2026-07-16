# Luma

Keyboard-first **personal** launcher (Rust CLI/TUI). Built for solo daily use — **not** for public release or distribution.

```bash
cd rust
cargo run -p luma                 # interactive TUI
cargo run -p luma -- query "app" --json
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```

## Scope

| In | Out |
| --- | --- |
| Local TUI + CLI on your Mac | App Store / notarized shipping |
| Apps, Windows, Clipboard, Notes, Quicklinks, Snippets, Wordbook, Projects, Records, SSH | Stub/unavailable feature expansion |
| Honest permission / unavailable states | Release soak, deny-as-policy, marketing docs |
| Module-local status and remediation rows | Centralized `doctor` command or diagnostics overlay |

## Docs

| Doc | Path |
| --- | --- |
| Operator guide | [`rust/README.md`](rust/README.md) |
| Module status | [`rust/docs/MODULES.md`](rust/docs/MODULES.md) |
| Empty Hub and keyboard behavior | [`rust/docs/hub.md`](rust/docs/hub.md) |
| Decisions | [`rust/docs/adr/`](rust/docs/adr/) |

## Data

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active app support |
| `~/Library/Logs/LumaNext/` | Runtime logs |

The empty Hub lists visible windows and modules. Press `1`–`9` to focus a numbered window;
digits remain ordinary prompt input in search fields. `win` provides the same shortcut in its
focused result list. `wb review due|new|wrong` starts the Wordbook review flow, while `rec`
searches the imported Records database.
