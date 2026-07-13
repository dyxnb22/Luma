# ADR-0002: Exclude luma-agent from the personal-workbench scope

- Status: Accepted
- Date: 2026-07-13

## Decision

**Do not build `luma-agent`.** Luma is a personal workbench, not an AI-agent product. This is a
product boundary, not merely a sequencing decision.

Codex and Claude Code may inform TUI ergonomics, but not conversational AI, autonomous planning,
tool orchestration, or a background process model.

Revisit this ADR only if the user explicitly chooses to change Luma's product boundary and one of
these needs is proven:

1. Background Clipboard/FSEvents while TUI is not running
2. Multiple CLI sessions sharing consistent live state
3. Global hotkey / external tools needing a long-lived process

## Consequences

- CLI and TUI use an in-process Engine.
- No Unix domain socket daemon, no LAN listener.
- If the boundary is explicitly changed, a new ADR must cover the product rationale, privacy,
  peer auth, single-instance, and version handshake before any daemon work begins.

## Non-goals

- Do not introduce an AI/LLM agent, chat UI, autonomous task planner, or tool-execution loop.
- Do not introduce an agent daemon for diagram completeness.
- Do not reintroduce GUI state machines to restore a global hotkey.
