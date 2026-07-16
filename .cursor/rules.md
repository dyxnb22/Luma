# Cursor Rules for Luma

Luma is a personal macOS Rust CLI/TUI workbench for solo daily use, not a public product and not
an AI-agent product. Implement under `rust/`. Follow `AGENTS.md` and
`rust/docs/GOVERNANCE.md` as the source of truth.

## Read first

1. `AGENTS.md`
2. `rust/README.md`
3. `rust/docs/GOVERNANCE.md`
4. `rust/docs/MODULES.md`
5. Relevant docs: `rust/docs/COMMAND_RECIPES.md`, `PROXY.md`, `SSH.md`, `hub.md`
6. `rust/docs/adr/`

## Product boundary

- Terminal CLI/TUI only; persist only under LumaNext.
- Codex and Claude Code are references for TUI ergonomics only. Do not introduce chat, LLM
  integration, autonomous planning/execution, tool loops, background agents/daemons, or
  multi-session orchestration unless the user explicitly changes the product boundary and an ADR
  records it.
- Do not add Web/Tauri/Electron, public-release machinery, plugin ABI, centralized `doctor`,
  `:doctor`, Doctor overlay, diagnostics export, or probe-port infrastructure.
- Keep Window layouts, Menu search, Browser tabs, and signed-host Translate deferred and out of
  the tree.
- Prioritize real friction in the registered modules: Apps, Windows, Proxy, Clipboard, Notes,
  Quicklinks, Snippets, Wordbook, Records, Projects, Command Recipes, SSH, Timers, and Secrets.

## Implementation rules

- Preserve crate boundaries and the architecture allowlist.
- `rust/bins/luma/src/compose.rs` is the sole module and adapter registration root.
- Modules use application ports; modules do not call macOS APIs or open stores directly.
- Keep TUI update/reducer and rendering pure. Effects own I/O and async work.
- Use structured `not_configured`, `unavailable`, `permission_required`, `empty`, and `failed`
  outcomes. No silent fallback or fake success.
- Destructive actions require confirmation; cancellation must actually stop or invalidate work.
- Tests must not steal focus or mutate the real clipboard. Isolate support/log roots and set
  environment overrides such as `SSH_CONFIG`.
- When a module is added, removed, renamed, its trigger/default changes, update
  `rust/docs/MODULES.md` and the root README In list in the same change.
- New modules should normally need only their module code, ports/adapters, one compose registration,
  tests, and docs. Avoid adding central engine/reducer/render branches; if that is unavoidable,
  record it as an extensibility cost.
- Apply `GOVERNANCE.md` soft file-size triggers only when touching an area. Do not split files for
  aesthetics or perform architecture cleanup without personal-use benefit.

## Audit mode

For a repository audit, inspect the complete project and report evidence rather than making a
quick checklist. Cover architecture, all registered modules, end-to-end business flows, TUI state
and stale async results, documentation drift, code smells, memory/task/FFI lifetimes, concurrency,
cancellation, shutdown/crash recovery, data consistency/migrations, security/privacy,
performance/backpressure, CLI/JSON compatibility, extensibility, and test quality.

Each finding must include P0-P3 severity, confidence (`confirmed`, `likely`, or `hypothesis`),
absolute file/line evidence, trigger, impact, root cause, suggested direction, and a regression
test. Distinguish production code from test code and separate confirmed facts from follow-up
hypotheses. On the first audit pass, do not edit or auto-fix files.

## Verification

Run from `rust/` when relevant:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
```

Do not default to release packaging, notarization, long soak/PTY evidence campaigns, or
cargo-deny policy.
