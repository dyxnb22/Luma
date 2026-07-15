# Agent notes (Luma)

Personal Mac workbench (Rust TUI). **Not for public release and not an AI-agent product.**

Product intent: a long-running, keyboard-first personal workbench — closer to Raycast plus
lightweight local modules than to an agent chat. Codex and Claude Code are interaction
references only (prompt editing, discovery, previews, command surfaces, and clear feedback);
do not copy their conversational-agent, autonomous-planning, or tool-orchestration product shape.

Prefer: fix friction in core modules (including Windows switcher, Wordbook, Projects, and Records), keep tests green, stay on LumaNext paths.
Avoid: release packaging, long soak campaigns, stub-module growth (Window layouts / Menu / Browser tabs remain stubs), architecture busywork without a personal benefit;
AI/LLM chat, autonomous agents, task planning/execution loops, background agent daemons, or multi-session orchestration
unless the user explicitly changes this product boundary.

Do not reintroduce a centralized `doctor` command, Doctor overlay, diagnostics export, or probe-port
subsystem. Modules must report permission, unavailable, and not-configured states locally.
