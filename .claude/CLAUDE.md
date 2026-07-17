# Claude Instructions for Luma

Luma is a personal macOS Rust CLI/TUI workbench for solo daily use. It is not a public product
and not an AI-agent product. Work under `rust/` and follow `AGENTS.md` plus
`rust/docs/GOVERNANCE.md`; those files are the source of truth when this file drifts.

## Read first

1. `AGENTS.md`
2. `rust/README.md`
3. `rust/docs/GOVERNANCE.md`
4. `rust/docs/MODULES.md`
5. Relevant module docs: `COMMAND_RECIPES.md`, `PROXY.md`, `SSH.md`, `hub.md`
6. `rust/docs/adr/`

## Product boundary

- Keep the product terminal CLI/TUI only, on LumaNext paths.
- Codex and Claude Code are TUI interaction references only: prompt editing, discoverability,
  previews, command surfaces, and clear feedback.
- Do not add chat, LLM integration, autonomous planning/execution, tool loops, background agent
  daemons, multi-session orchestration, Web/Tauri/Electron, or a plugin ABI without an explicit
  product-boundary ADR.
- Do not reintroduce centralized `doctor`, `:doctor`, a Doctor overlay, diagnostics export, or
  probe-port infrastructure. Modules own their permission, unavailable, and not-configured rows.
- Keep Window layouts, Menu search, Browser tabs, and signed-host Translate deferred; do not grow
  them as half-implemented stubs.
- Prefer real daily-use friction fixes in Apps, Windows, Proxy, Clipboard, Notes, Quicklinks,
  Snippets, Wordbook, Projects, Records, Command Recipes, SSH, Timers, and Secrets.
- Interactive commands require a leading `/` (`/ssh`, `/rec browse`, `/cmd test`, `/settings`,
  `/help`). Unprefixed input is global search; legacy bare-trigger and colon command forms are
  retired.

## Architecture and data rules

- `rust/bins/luma/src/compose.rs` is the sole composition and module-registration root.
- Preserve crate layering and keep platform I/O behind application ports.
- Keep TUI update/reducer and render logic pure; effects own I/O and async work.
- Persist only in LumaNext. Tests must isolate with `LUMA_NEXT_SUPPORT_DIR`,
  `LUMA_NEXT_LOGS_DIR`, and relevant environment overrides such as `SSH_CONFIG`.
- Keep `not_configured`, `unavailable`, `permission_required`, `empty`, and `failed` distinct.
  Never turn an error into a silent empty or fake success.
- Destructive actions require confirmation and cancellation must be real.
- When adding or changing a module, update `rust/docs/MODULES.md` and the root README In list in
  the same change.
- New module work should normally require a module, its ports/adapters, one explicit compose
  registration, tests, and docs. Avoid edits to central engine/reducer/render matches; if they are
  required, treat that as an extensibility finding.
- Apply the governance soft file-size triggers when touching hot paths. Do not split files purely
  for line-count aesthetics.

## Audit mode

When the user asks for an audit, inspect the whole repository before proposing changes. Cover
architecture, every registered module, end-to-end business flows, TUI state transitions,
documentation drift, code smells, memory/task/resource lifetimes, concurrency and cancellation,
crash recovery, migrations, security/privacy, performance/backpressure, CLI/JSON compatibility,
macOS FFI, extensibility, and test quality.

Audit findings must include severity P0-P3, confidence (`confirmed`, `likely`, or `hypothesis`),
absolute file/line evidence, trigger, impact, root cause, suggested direction, and a regression
test. Separate production findings from test-only patterns and do not report unsupported guesses
as facts. In a first audit pass, do not modify files or auto-fix findings.

## Verification

From `rust/`, use the personal verification set:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p luma --test cli_blackbox
./scripts/check_architecture.sh
```

Do not turn release packaging, notarization, long soak campaigns, or cargo-deny policy into
default work.
