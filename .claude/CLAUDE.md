# Claude Instructions for Luma

Review the **Rust CLI/TUI** under `rust/`.

## Read first

1. `rust/README.md`
2. `rust/docs/MODULES.md`
3. `rust/docs/adr/`

## Focus

- Engine / CLI / TUI contracts and crate boundaries.
- LumaNext data isolation; honest unavailable / permission / not-configured states.
- No silent fallbacks; no foreground automation in tests/soak.
- Finish quality over new modules.
