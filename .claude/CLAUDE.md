# Claude Instructions for Luma

**Personal-use only** — not a public product. Review/implement the Rust CLI/TUI under `rust/`.

## Read first

1. `rust/README.md`
2. `rust/docs/MODULES.md`
3. `rust/docs/SSH.md` (when touching `luma.ssh`)
4. `rust/docs/adr/`

## Focus

- Daily-driver quality for Apps / Clipboard / Notes / Quicklinks / Snippets / Windows / Wordbook / Projects / Records / SSH.
- Engine / CLI / TUI contracts and crate boundaries.
- LumaNext data isolation; honest unavailable / permission / not-configured states.
- No centralized `doctor` command/overlay or diagnostics export; keep status and remediation rows module-local.
- No silent fallbacks; no foreground automation in tests.
- Do **not** prioritize release gates, soak evidence packs, notarization, or stub-module expansion.
