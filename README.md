# Luma

Keyboard-first **personal** launcher (Rust CLI/TUI). Built for solo daily use — **not** for public release or distribution.

```bash
cd rust
cargo run -p luma                 # interactive TUI
cargo run -p luma -- query "app" --json
cargo run -p luma -- doctor --json
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```

## Scope

| In | Out |
| --- | --- |
| Local TUI + CLI on your Mac | App Store / notarized shipping |
| Apps, Clipboard, Notes, Quicklinks, Snippets | Stub/unavailable feature expansion |
| Honest permission / unavailable states | Release soak, deny-as-policy, marketing docs |

## Docs

| Doc | Path |
| --- | --- |
| Operator guide | [`rust/README.md`](rust/README.md) |
| Module status | [`rust/docs/MODULES.md`](rust/docs/MODULES.md) |
| Decisions | [`rust/docs/adr/`](rust/docs/adr/) |

## Data

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active app support |
| `~/Library/Logs/LumaNext/` | Logs / diagnostics |
