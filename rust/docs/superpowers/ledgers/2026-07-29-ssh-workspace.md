# SSH Workspace Ledger

**Plan:** `rust/docs/superpowers/plans/2026-07-29-ssh-workspace.md`
**Branch:** `cursor/ssh-workspace-a317`
**Base:** `codex/ssh-workspace`

| Task | Status | Notes |
| --- | --- | --- |
| 1 Embedded PTY port | done | portable-pty + Fake; process-group kill |
| 2 vt100 screen projection | done | scrollback 2000, OSC52 inert; flood/resize tests |
| 3 Workspace route + state machine | done | EmbeddedTerminal outcome; full-width until shelf |
| 4 Input routing / security | done | leader chords + shelf Esc |
| 5 Static command shelf | done | Copy/Insert + SSH ops + favorites ★ |
| 6 Recipe scope/params | done | ssh_session / remote_shell |
| 7 Wire shelf + forms | done | Tab/Shift+Tab params; meta favorite/use_count |
| 8 Docs/default/E2E | done | ADR-0008 + module docs + stability tests |
