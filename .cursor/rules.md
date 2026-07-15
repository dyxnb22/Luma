# Cursor Rules for Luma

**Personal-use only** (see `.cursor/rules/personal-use.mdc`). Implement the Rust CLI/TUI under `rust/`. Do not drive work toward public release.

Luma is a personal workbench, not an AI-agent product. Use Codex and Claude Code only as TUI
interaction references: prompt editing, keyboard discoverability, previews, command surfaces, and
clear status feedback. Do not introduce chat, LLM integration, autonomous planning/execution,
tool loops, or background-agent/daemon infrastructure unless the user explicitly changes scope.

## Read first

1. `rust/README.md`
2. `rust/docs/MODULES.md`
3. `rust/docs/adr/`

## Rules

- One scoped task at a time; match existing crate boundaries.
- Persist only under LumaNext (`LUMA_NEXT_*` in tests).
- No silent fallbacks: use structured unavailable / permission / not-configured outcomes.
- Do not reintroduce `luma doctor`, `:doctor`, a Doctor overlay, diagnostics export, or probe-port subsystem.
- No foreground automation in tests (`open`, osascript, AX paste, system clipboard mutation).
- Do not expand stub modules or add release/CI/soak/notarization machinery unless asked.
- Do not add AI/LLM agent features, conversational workflows, autonomous task execution, or
  multi-session/background-agent infrastructure unless asked.

## Verification

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```
