# Luma (Rust CLI/TUI)

## Commands

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
# optional: cargo deny check

cargo run -p luma -- query "app safari" --json
cargo run -p luma -- query "clip" --json
cargo run -p luma -- query "n" --json
cargo run -p luma -- modules list --json
cargo run -p luma -- doctor --json
cargo run -p luma -- config get --json
cargo run -p luma -- config set --notes-root /path/to/notes
cargo run -p luma -- action run --query "fake hello" --action-id open --json
cargo run -p luma   # interactive TUI
```

See [`docs/MODULES.md`](docs/MODULES.md) for module status.

## Soak

```bash
cd rust
bash scripts/soak_runner.sh 100
LUMA_SOAK_ROOT=/tmp/luma-soak bash scripts/soak_runner.sh 100
```

Records exit codes/durations in `state.tsv`. Does not claim multi-day observation without wall-clock evidence.

## Data roots

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active |
| `~/Library/Logs/LumaNext/` | Logs / diagnostics |

Tests must use tempfile + `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`. Do not mutate a developer’s real LumaNext store from automation unless the user asks.

Optional importers: `luma migrate …` with an explicit legacy path (dry-run by default). Never write into the legacy tree.
