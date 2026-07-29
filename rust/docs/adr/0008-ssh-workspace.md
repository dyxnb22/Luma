# ADR-0008: SSH Workspace (embedded terminal)

## Status

Accepted (personal daily-driver)

## Context

SSH previously suspended the Ratatui TUI and handed the whole terminal to
`/usr/bin/ssh`. That worked, but blocked Luma chrome (status, commands, return
path) for the duration of the session.

## Decision

SSH Connect opens an **SSH Workspace** inside the Luma TUI:

- Single active embedded child PTY (`portable-pty`) running `/usr/bin/ssh`
- `vt100` projects ANSI into Ratatui cells
- Command Shelf reuses Command Recipes (`scope=ssh_session`,
  `target=remote_shell`) for Copy / Insert only (no auto-Enter); native reconnect/disconnect
  rows are explicit actions. It starts visible beside the PTY when the terminal is wide enough
- Passwords stay on the existing Keychain AskPass path
- **Compat mode** (`Connect (compat mode)` / `l`) keeps the legacy
  suspend→ssh→resume flow

The Swift workbench host (ADR-0007) remains a thin outer PTY: no product
sidebar, no module data, no second UI.

## Consequences

- Default Connect no longer leaves Ratatui
- A clean remote `exit 0` records the session and returns to `/ssh`; failed exits retain the
  final screen for diagnosis
- Layout width rules drive side / overlay / full-page shelf; PTY resize follows
- `F6` owns shelf focus; `Ctrl+Space` owns terminal leader chords, avoiding shortcut ambiguity
- Remote cursor/input modes, bracketed paste, and common xterm key sequences are projected
- Remote output never enters SQLite / Records / search
- OSC 52 and title sequences are not applied to the host
- Scrollback is hard-capped at 2000 lines and browsed with Option+Up/Down or Fn+Up/Down on Mac
  (PageUp/PageDown works on full keyboards)
- The command shelf uses collapsed category rows plus an all-category incremental search, so
  growth in built-in or personal recipes does not turn the side pane into an unbounded flat list
- Phase 5 ideas (mouse, Docker discovery, SFTP browser) stay out of tree until needed
