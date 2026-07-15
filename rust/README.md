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
printf '%s' 'secret' | cargo run -p luma -- secrets set my-label
cargo run -p luma -- modules list --json
cargo run -p luma -- doctor --json
cargo run -p luma -- config get --json
cargo run -p luma -- config set --notes-root /path/to/notes
cargo run -p luma -- config set --projects-root ~/dev
cargo run -p luma -- config set --notes-exclude 'private/*'
cargo run -p luma -- config set --clear-notes-excludes
cargo run -p luma   # interactive TUI
```

Optional local hygiene: `bash scripts/check_architecture.sh`.

**Fixtures:** `fixtures/notes-workspaces/` for Notes scan/index tests; `fixtures/legacy/` for migrate blackbox.

See [`docs/MODULES.md`](docs/MODULES.md) for module status.

## Data roots

| Path | Role |
| --- | --- |
| `~/Library/Application Support/LumaNext/` | Active settings / stores |
| `~/Library/Application Support/LumaNext/diagnostics/` | Diagnostic exports |
| `~/Library/Logs/LumaNext/` | Logs |

Tests must use tempfile + `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`.

Optional importers: `luma migrate …` with an explicit legacy path (dry-run by default).
