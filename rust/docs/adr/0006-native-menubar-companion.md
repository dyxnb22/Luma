# ADR-0006: Native menu bar companion

- Status: Accepted
- Date: 2026-07-18

## Decision

Luma may have a small native macOS menu bar companion for glanceable Wordbook status,
visible-window switching, and safe entry points into the existing terminal TUI.

This is a second executable and UI entry point, not a second module registry or application
engine. `bins/luma` remains the only module-registration composition root. `luma-menubar` must
not depend on `luma-tui`, `luma-modules`, `luma-protocol`, or the full registry/Engine startup.

The companion uses `NSStatusItem`/`NSMenu`, shares only existing LumaNext data, and keeps all
long-running business work in the terminal application. It does not add menu search, a global
hotkey, a popover, a daemon, a Unix socket, an agent, or background Clipboard/Notes/Timers work.

The first version includes:

- Wordbook due/today summary and a link that pre-fills `/wb review due` in the TUI;
- up to five visible windows using the existing window port, with local permission/unavailable
  states;
- opening `luma`, opening it with `/settings` pre-filled, refresh, launch-at-login, and quit.

All companion reads are non-initializing: absent settings or Wordbook data is reported as
`not_configured`; corrupt or unavailable data is not repaired by the companion. TUI launch
queries are editable prompt text and are never submitted automatically.

## Consequences

- `bins/luma-menubar` is a separate AppKit main-loop process and has its own macOS TCC identity.
- GUI dependencies are confined to the companion binary; the `luma` dependency tree remains
  terminal-only.
- Shared settings remain under the existing lock/CAS semantics for writes. Companion reads use
  explicit existing-only APIs and do not create defaults or schema.
- Login launch is an explicit user choice backed by `SMAppService.mainApp` in a local app bundle;
  no LaunchAgent or KeepAlive files are written by Luma.
- Timers retain their current meaning: quitting the terminal app pauses running timers.

## Verification

Architecture checks must assert the dependency boundary, and tests must cover missing,
corrupt, and unavailable local data without destructive writes. Manual macOS checks cover the
menu bar lifecycle, Accessibility behavior, Terminal launch, and login-item toggle.
