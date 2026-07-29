# ADR-0006: Native menu bar companion

- Status: Superseded by [ADR-0007](0007-native-workbench-host.md)
- Date: 2026-07-18
- Superseded: 2026-07-26

## Historical decision

Luma briefly included a small native macOS menu-bar companion for glanceable Wordbook status,
visible-window switching, and entry points into the terminal TUI. It was a separate Rust/AppKit
executable with its own bundle identity and permissions.

## Superseding decision

The companion is removed from the product and source tree. The thin native PTY host from
ADR-0007 now provides the system-level entry point through a configurable global shortcut, while Wordbook status,
window switching, settings, and every other module action stay in the Rust TUI.

Maintaining two native entry points added installation, permission, login-item, lifecycle, and
documentation cost without enough daily-use value. The menu-bar snapshot also duplicated a small
projection of product state, whereas the PTY host exposes the complete existing workbench without
duplicating business UI.

The removed artifacts were `bins/luma-menubar`, `scripts/build_menubar_app.sh`, and
`scripts/menubar-Info.plist`. They must not be restored unless a later ADR identifies a distinct
personal-use need that cannot be served by the TUI or the workbench hotkey.
