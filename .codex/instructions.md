# Codex Instructions for Luma

Implement the **Rust CLI/TUI** under `rust/`.

## Read first

1. `rust/README.md`
2. `rust/docs/MODULES.md`
3. `rust/docs/adr/`

## Rules

- Persist only under LumaNext (`LUMA_NEXT_*` in tests).
- Prefer structured failures over empty results that hide denials.
- No foreground automation in tests/soak.

## Verification

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```
