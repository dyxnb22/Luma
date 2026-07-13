# ADR-0001: CLI/TUI product shape

- Status: Accepted
- Date: 2026-07-13

## Decision

Luma is a **pure interactive CLI/TUI**. Product code lives under `rust/`. We do not ship Web, Tauri, Electron, or native GUI product UI.

### Product entry

- Primary: `luma` opens a long-running TUI in the terminal.
- Non-interactive siblings share the same application API: `luma query`, `luma action run`, `luma doctor`, `luma config`, `luma modules list` (stable `--json` where applicable).
- Global hotkey / menu bar / floating panel are out of scope for the current product.

### Architecture

- In-process Engine: TUI and engine share a process via a stable Command/Event boundary (`luma-protocol`).
- `luma-agent` is deferred (see ADR-0002).
- Layering: `platform` → `storage` → `domain` ← `application` ← `modules`; `luma-tui` depends on application projections/ports only; `bins/luma` is the composition root.
- TUI: one `AppState`, one event loop, Elm-style `update` / `render`. `update` and `render` must not perform I/O; Effects own I/O and return messages.
- Failure taxonomy distinguishes permission denied, cold/warming cache, not configured, timeout, cancelled, and true empty results.

### Non-goals

- Web admin, Chrome management pages, Tauri/Electron/GUI product shells.
- Writing into any path except LumaNext (and tempfile overrides) unless the user runs an explicit migrate importer.

## Consequences

- Module behavior is tracked in [`../MODULES.md`](../MODULES.md).
- macOS capabilities stay in `luma-platform-macos` (see ADR-0003).
- A thin signed host may be proposed later only with spike evidence — product UI remains TUI.
