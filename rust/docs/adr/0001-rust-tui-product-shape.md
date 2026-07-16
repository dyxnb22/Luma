# ADR-0001: CLI/TUI product shape

- Status: Accepted
- Date: 2026-07-13

## Decision

Luma is a **personal** interactive CLI/TUI under `rust/`: a long-running, keyboard-first
workbench for local modules and personal information. It is **not** a public-release product and
it is **not an AI-agent product**. We do not ship Web, Tauri, Electron, native GUI, or
distribution/notarization workflows.

Codex and Claude Code are references for interaction quality — prompt editing, keyboard
discoverability, previews, command surfaces, and clear progress/failure feedback — not a product
template. Luma does not adopt their conversational, autonomous-planning, LLM, or tool-loop model.

### Product entry

- Primary: `luma` opens a long-running TUI in the terminal.
- Non-interactive siblings share the same application API: `luma query`, `luma action run`, `luma config`, `luma modules list` (stable `--json` where applicable).
- Global hotkey / menu bar / floating panel are out of scope.

### Architecture

- In-process Engine: TUI and engine share a process via a stable Command/Event boundary (`luma-protocol`).
- No `luma-agent`, AI agent, or background daemon is part of this product shape (see ADR-0002).
- Layering: `platform` → `storage` → `domain` ← `application` ← `modules`; `luma-tui` depends on application projections/ports only; `bins/luma` is the composition root.
- `luma-application` may depend on `luma-storage` for settings/path adapters at composition time (`compose.rs`), not opened ad hoc inside the engine.
- TUI: one `AppState`, one event loop, Elm-style `update` / `render`. `update` and `render` must not perform I/O; Effects own I/O and return messages.
- Failure taxonomy distinguishes permission denied, cold/warming cache, not configured, timeout, cancelled, and true empty results.

### Non-goals

- Public release, App Store, notarization, multi-user product packaging.
- Web admin, Chrome management pages, Tauri/Electron/GUI product shells.
- AI/LLM chat, autonomous planning or execution, agent tool loops, and background-agent or
  multi-session orchestration infrastructure.
- Stub modules (Media, Window layouts, Menu search, Browser tabs) and signed-host Translate.
  Personal Wordbook (`luma.wordbook`: vocab + SRS, no desktop pet) is allowed.
- Persistent Luma-owned state lives under LumaNext (and tempfile overrides in tests). Explicit
  user actions may operate on an imported or configured workspace path when that module defines
  the boundary (for example Notes and Projects), or use a narrowly scoped platform adapter for
  an opt-in system integration such as Keychain or system-proxy ownership. Those adapters must
  preserve their local safety, ownership, and rollback rules; modules do not acquire broader
  filesystem or system-write access.

Personal **Windows** (`luma.windows`: list + focus, Hub projection of previous-frontmost)
is allowed; see [0004-windows-hub-projection.md](./0004-windows-hub-projection.md).

Personal **Projects** (explicit imported directories only) and **Records** (local SQLite media
log with read-only Markdown import) are allowed; their data and mutation boundaries are defined
in [0005-records-and-imported-projects.md](./0005-records-and-imported-projects.md).

## Consequences

- Module behavior is tracked in [`../MODULES.md`](../MODULES.md).
- macOS capabilities stay in `luma-platform-macos` (see ADR-0003).
- A signed host is out of scope for personal use.
