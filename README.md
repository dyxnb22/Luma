# Luma

Keyboard-first personal launcher as a **Rust CLI/TUI**.

```bash
cd rust
cargo run -p luma                 # interactive TUI
cargo run -p luma -- query "app" --json
cargo run -p luma -- doctor --json
cargo test --workspace --all-features
```

## Docs

| Doc | Path |
| --- | --- |
| Operator guide | [`rust/README.md`](rust/README.md) |
| Modules | [`rust/docs/MODULES.md`](rust/docs/MODULES.md) |
| ADRs | [`rust/docs/adr/`](rust/docs/adr/) |

## Data

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active app support |
| `~/Library/Logs/LumaNext/` | Logs / diagnostics |

Override in tests with `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`. Optional `luma migrate …` importers read an older Application Support tree only when you pass an explicit path; they never write there.
