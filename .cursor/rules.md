# Cursor Rules for Luma

Implement the **Rust CLI/TUI** under `rust/`.

## Read first

1. `rust/README.md`
2. `rust/docs/MODULES.md`
3. `rust/docs/adr/`

## Rules

- One scoped task at a time; match existing crate boundaries.
- Persist only under LumaNext (`LUMA_NEXT_*` in tests).
- No silent fallbacks: use structured unavailable / permission / not-configured outcomes.
- No foreground automation in tests/soak (`open`, osascript, AX paste, system clipboard mutation).

## Verification

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```
