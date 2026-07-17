# Codex Instructions for Luma

**Personal-use only.** Implement the Rust CLI/TUI under `rust/`. No public-release scope.

Luma is a personal workbench, not an AI-agent product. Treat Codex and Claude Code as TUI
interaction references only: keyboard editing, discoverability, previews, command surfaces, and
status feedback are in scope; chat, LLM integration, autonomous planning/execution, tool loops,
and a background agent/daemon are not, unless the user explicitly changes this boundary.

## Read first

1. `rust/README.md`
2. `rust/docs/MODULES.md`
3. `rust/docs/SSH.md` (when touching `luma.ssh`)
4. `rust/docs/adr/`

## Rules

- Persist only under LumaNext (`LUMA_NEXT_*` in tests).
- Prefer structured failures over empty results that hide denials.
- Do not reintroduce `luma doctor`, `:doctor`, a Doctor overlay, diagnostics export, or probe-port subsystem.
- No foreground automation in tests.
- Do not add release/CI/soak/notarization work or revive stub modules unless asked.
- Do not add AI/LLM agent features, conversational workflows, autonomous task execution, or
  multi-session/background-agent infrastructure unless asked.
- Interactive prompt commands require `/` (`/ssh`, `/rec browse`, `/cmd test`, `/settings`,
  `/help`). Unprefixed text is global search; do not reintroduce bare-trigger or colon command
  forms.

## Verification

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
```
