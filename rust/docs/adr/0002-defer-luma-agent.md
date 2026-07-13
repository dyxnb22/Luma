# ADR-0002: Defer luma-agent

- Status: Accepted
- Date: 2026-07-13

## Decision

**Skip `luma-agent` for now.**

Revisit only when one of these is proven:

1. Background Clipboard/FSEvents while TUI is not running
2. Multiple CLI sessions sharing consistent live state
3. Global hotkey / external tools needing a long-lived process

## Consequences

- CLI and TUI use an in-process Engine.
- No Unix domain socket daemon, no LAN listener.
- A future Agent ADR must cover peer auth, single-instance, and version handshake.

## Non-goals

- Do not introduce Agent for diagram completeness.
- Do not reintroduce GUI state machines to restore a global hotkey.
