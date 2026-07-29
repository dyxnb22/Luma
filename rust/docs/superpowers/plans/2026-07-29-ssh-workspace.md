# SSH Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace temporary whole-window SSH handoff with an in-TUI SSH Workspace (single session, optional Command Recipes shelf with Copy/Insert only), keeping compat mode and Keychain askpass.

**Architecture:** Host SwiftTerm PTY still wraps the whole Luma TUI (ADR-0007). Inside Ratatui, a new `Route::SshWorkspace` owns an embedded child PTY (`portable-pty`) running `/usr/bin/ssh`, parsed with `vt100` into a cell grid. Command Shelf reuses Command Recipes (`scope=ssh_session`, `target=remote_shell`) for Copy/Insert. Platform adapter owns PTY I/O; TUI reducer/render stay pure.

**Tech Stack:** Rust, portable-pty, vt100, Ratatui/crossterm, existing Command Recipes TOML, macOS Keychain askpass (unchanged).

## Global Constraints

- Personal Mac workbench only; no release packaging, no multi-session tabs, no auto-execute Enter.
- ADR-0007: Swift host stays window/PTY/lifecycle only — no sidebar or product UI.
- Passwords stay in Keychain via existing `SSH_ASKPASS` / `LUMA_SSH_ASKPASS_ACCOUNT`.
- Remote output never enters SQLite / Records / search index; scrollback hard-capped at 2000 lines.
- Forbid OSC 52 clipboard, remote notification, title sequences affecting host.
- GOVERNANCE 7a: prefer `ActionOutcome` + TUI Effects + ports; avoid new Engine `Command`/`Event` arms.
- Register adapters only in `bins/luma/src/compose.rs`.
- Persist only under LumaNext (`LUMA_NEXT_*` in tests).
- Verify with: `cargo fmt`, `clippy -D warnings`, `cargo test --workspace --all-features`, `cli_blackbox`, `./scripts/check_architecture.sh`.
- Branch: `cursor/ssh-workspace-a317` off `codex/ssh-workspace`. Phase 5 (mouse, host-specific groups, Docker discovery, SFTP browser) is explicitly out of scope.

## File Map

| Path | Responsibility |
| --- | --- |
| `luma-application/src/ports/embedded_pty.rs` | `EmbeddedPtyPort` + session handle + Fake |
| `luma-application/src/embedded_terminal.rs` | Request/types for embedded spawn (program/args/env/size) |
| `luma-platform-macos/src/embedded_pty.rs` | `MacEmbeddedPty` via `portable-pty` |
| `luma-tui/src/ssh_workspace/` | State machine, ANSI projection, input routing, shelf UI |
| `luma-tui/src/reducer/ssh_workspace.rs` | Pure reducer for workspace msgs |
| `luma-tui/src/render/ssh_workspace.rs` | Layout: header + terminal + optional shelf |
| `luma-domain/src/recipe.rs` | `RecipeScope::SshSession`, `RecipeTarget`, parameters |
| `luma-storage/src/command_recipes_*.rs` | Parse/validate new fields + SSH builtins |
| `luma-modules/src/ssh/` | Default connect → workspace; compat action |
| Docs: `SSH.md`, `COMMAND_RECIPES.md`, `MODULES.md`, ADR-0008 |

---

### Task 1: Embedded PTY port + portable-pty adapter (Phase 0)

**Files:**
- Create: `rust/crates/luma-application/src/ports/embedded_pty.rs`
- Create: `rust/crates/luma-platform-macos/src/embedded_pty.rs`
- Modify: `rust/crates/luma-application/src/ports/mod.rs`
- Modify: `rust/crates/luma-platform-macos/src/lib.rs`
- Modify: `rust/Cargo.toml` (add `portable-pty` workspace dep)
- Modify: `rust/crates/luma-platform-macos/Cargo.toml`
- Modify: `rust/bins/luma/src/compose.rs` (wire later in Task 3 if TUI needs it; export type now)
- Test: unit tests in `embedded_pty.rs` (Fake) + platform integration test spawning `/bin/sh`

**Interfaces:**
- Produces:
```rust
pub struct EmbeddedPtySize { pub cols: u16, pub rows: u16 }
pub struct EmbeddedPtySpawnRequest {
    pub program: String,
    pub args: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub size: EmbeddedPtySize,
}
pub enum EmbeddedPtyEvent {
    Output(Vec<u8>),
    Exited { code: Option<i32> },
}
pub trait EmbeddedPtySession: Send {
    fn writer(&self) -> Box<dyn std::io::Write + Send>;
    fn resize(&self, size: EmbeddedPtySize) -> Result<(), EmbeddedPtyError>;
    fn kill(&self) -> Result<(), EmbeddedPtyError>;
}
pub trait EmbeddedPtyPort: Send + Sync {
    fn spawn(&self, request: EmbeddedPtySpawnRequest)
        -> Result<(Box<dyn EmbeddedPtySession>, mpsc::Receiver<EmbeddedPtyEvent>), EmbeddedPtyError>;
}
```

- [x] **Step 1: Write failing Fake + trait tests** for spawn/write/resize/kill/exit semantics (Fake in application port).
- [x] **Step 2: Run tests — expect fail** (module missing).
- [x] **Step 3: Implement port + Fake + `MacEmbeddedPty` with `portable-pty`.** Reader thread sends `Output` / `Exited`. Process group kill on `kill()`. Bounded channel.
- [x] **Step 4: Integration test** spawn `/bin/sh -c 'printf hello; exit 0'`, assert output contains `hello` and exit 0. Skip gracefully if PTY unavailable.
- [x] **Step 5: Commit** `feat(pty): add EmbeddedPtyPort with portable-pty adapter`

---

### Task 2: vt100 screen projection helpers (Phase 0)

**Files:**
- Create: `rust/crates/luma-tui/src/ssh_workspace/screen.rs`
- Create: `rust/crates/luma-tui/src/ssh_workspace/mod.rs`
- Modify: `rust/crates/luma-tui/src/lib.rs` (or `mod.rs`) to expose `ssh_workspace`
- Modify: `rust/crates/luma-tui/Cargo.toml` — add `vt100` dep
- Test: unit tests in `screen.rs`

**Interfaces:**
- Produces: `VtScreen { parser: vt100::Parser }` with `feed(&[u8])`, `resize(cols,rows)`, `cells() -> Vec<Vec<Cell>>`, `cursor()`, `in_alternate_screen()`, scrollback cap 2000 via parser rows+scrollback.

- [x] **Step 1: Write failing tests** for ANSI color, cursor, alternate screen, scrollback cap, emoji/CJK width.
- [x] **Step 2: Run — expect fail.**
- [x] **Step 3: Implement projection** mapping vt100 cells → ratatui `Cell`/`Span` styles (fg/bg/bold/underline). Strip/ignore OSC 52 and title sequences at feed boundary when detectable; never write pasteboard from parser.
- [x] **Step 4: Tests pass. Commit** `feat(tui): add vt100 screen projection for embedded terminal`

---

### Task 3: SSH Workspace route, state machine, effects (Phase 1)

**Files:**
- Create: `rust/crates/luma-tui/src/ssh_workspace/state.rs`, `input.rs`, `layout.rs`
- Create: `rust/crates/luma-tui/src/reducer/ssh_workspace.rs`
- Create: `rust/crates/luma-tui/src/render/ssh_workspace.rs`
- Modify: `view_model/input.rs` (`Route::SshWorkspace`, `FocusZone::Terminal` / `CommandShelf`)
- Modify: `view_model/surfaces.rs` (`SshWorkspaceState`)
- Modify: `effect.rs`, `msg.rs`, `app.rs`, `reducer/dispatch.rs`, `render/mod.rs`
- Modify: `luma-application/src/module.rs` + `luma-protocol` `ActionOutcomeDto` — add `EmbeddedTerminal { program, args, environment, record_alias, title }`
- Modify: engine action mapping + TUI reducer/engine bridge
- Modify: `bins/luma/src/main.rs` / TUI entry to inject `Arc<dyn EmbeddedPtyPort>`
- Modify: `luma-modules/src/ssh/mod.rs` — primary `connect` returns `EmbeddedTerminal`; add `connect_compat` → `InteractiveTerminal`
- Test: reducer unit tests for Starting→Connected→Failed/Disconnected; layout width thresholds

**State machine:** `Starting | Authenticating | Connected | Disconnected | Failed` with header status strings from the product spec.

**Effects:**
```rust
StartEmbeddedTerminal { ... }
WriteEmbeddedPty { bytes: Vec<u8> }
ResizeEmbeddedPty { cols, rows }
KillEmbeddedPty
```

Layout: ≥118 cols → terminal + shelf (shelf hidden until Phase 2 but layout reserve optional); 80–117 full terminal; <80 full terminal. Header always shows alias · user@host:port · status. Footer hints.

- [x] **Step 1: Failing reducer/layout tests.**
- [x] **Step 2: Implement route + effects + app I/O loop** (async reader → `Msg::SshPtyOutput` / `SshPtyExited`). On exit non-zero keep last screen; keys `r` reconnect, `Esc` back to `/ssh `, `l` compat, `c` copy error summary.
- [x] **Step 3: Wire SSH connect → EmbeddedTerminal**; keep `connect_compat` / action "Connect (compat mode)".
- [x] **Step 4: Unit tests green. Commit** `feat(ssh): embed SSH sessions in TUI workspace route`

---

### Task 4: Input routing, resize, reconnect, security filters (Phase 1)

**Files:**
- Modify: `ssh_workspace/input.rs`, `app.rs`, `terminal.rs` if needed
- Test: input unit tests; OSC52 ignored test

`F6` opens/focuses the shelf. `Ctrl+Space` arms the terminal leader;
`Ctrl+Space` then `Space` sends raw Ctrl+Space to PTY, `f` toggles fullscreen,
`d` confirms disconnect, `r` reconnects, and `q` leaves. When focus is Terminal, keys go to
the PTY (including Ctrl+C and Esc). Esc from the shelf returns to the terminal. On quit of Luma,
kill the PTY process group and wait.

- [x] **Step 1: Failing input/routing tests.**
- [x] **Step 2: Implement. Commit** `feat(ssh): workspace keyboard leader and PTY input routing`

---

### Task 5: Static Command Shelf UI (Phase 2)

**Files:**
- Create: `ssh_workspace/shelf.rs`
- Modify: render/reducer for shelf focus, search `/`, favorites
- SSH-native ops (not recipes): copy alias/IP/ssh/sftp cmd, show info, reconnect, disconnect, copy SFTP command
- Built-in static remote commands (hardcode list matching System/Docker/SSH defaults until Task 6 loads recipes)
- Actions: `c` copy via PasteboardPort effect, `i` insert into PTY without Enter, Enter preview only
- Width: ≥118 side panel 36–44 cols (terminal ≥72); 80–117 overlay; <80 full-page shelf

- [x] **Step 1: Failing shelf navigation/copy/insert tests** (insert bytes must not end with `\r`/`\n`).
- [x] **Step 2: Implement. Commit** `feat(ssh): add static command shelf with copy/insert`

---

### Task 6: Recipe scope/target/parameters (Phase 3)

**Files:**
- Modify: `luma-domain/src/recipe.rs`
- Modify: `luma-storage/src/command_recipes_config.rs`, `command_recipes_builtin.rs`
- Modify: `recipe_environment.rs` `recipe_in_scope` for `SshSession`
- Add parameter types: text/integer/choice/boolean/path with validation rules from spec
- Substitution: `${param}` must be whole arg token; shell-quote each; forbid program change; forbid shell -c; no secret type; no persist drafts
- Context vars: `${ssh.alias}`, `${ssh.hostname}`, `${ssh.user}`, `${ssh.port}`
- Builtin SSH-session recipes for System/Docker groups; mark restart etc. confirm/destructive
- Backward compatible: missing `target` → `local_shell`; missing `scope` unchanged; old TOML still loads

- [x] **Step 1: Failing domain/storage tests** for parse, validation, quote safety, backward compat.
- [x] **Step 2: Implement. Commit** `feat(recipes): ssh_session scope, remote_shell target, parameters`

---

### Task 7: Wire shelf to recipes + parameter form (Phase 3)

**Files:**
- Modify: shelf to load `scope=ssh_session` recipes from catalog (via injected snapshot or module helper; avoid new Engine Command — load catalog through existing repo port passed into TUI or resolve at connect time into workspace state)
- Parameter form: Tab/Shift+Tab, preview before copy/insert
- Favorites/use_count reuse existing meta sqlite (global per recipe id is OK for v1)

- [x] **Step 1: Failing form/preview tests.**
- [x] **Step 2: Implement. Commit** `feat(ssh): recipe-backed shelf with parameter forms`

---

### Task 8: Docs, settings default, ADR, E2E tests (Phase 4)

**Files:**
- Create: `rust/docs/adr/0008-ssh-workspace.md`
- Update: `SSH.md`, `COMMAND_RECIPES.md`, `MODULES.md`, root `README.md` if In/Out changes
- Settings: SSH Workspace default on; compat still available
- Tests: flood output memory bound; resize storm; process cleanup; fake sshd or `/bin/sh` stand-in E2E where macOS sshd unavailable on Linux CI
- Architecture script still green

- [x] **Step 1: Docs + default enable.**
- [x] **Step 2: Stability tests.**
- [x] **Step 3: Full verify suite.**
- [x] **Step 4: Commit** `docs(ssh): document SSH Workspace and enable by default`

---

## Spec Coverage Checklist

- [x] In-TUI SSH, single session — Tasks 3–4
- [x] Layout width rules + PTY resize — Tasks 3–5
- [x] State machine + header status — Task 3
- [x] Compat mode — Task 3
- [x] Leader keys / shelf keys — Tasks 4–5
- [x] Recipes shelf Copy/Insert only — Tasks 5–7
- [x] Parameters + safety — Task 6
- [x] Security (Keychain, no OSC52, scrollback, kill on exit) — Tasks 1,2,4,8
- [x] No mouse / no Phase 5 — explicit out of scope
- [x] ADR-0007 preserved — Task 8 ADR-0008
