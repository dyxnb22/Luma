# ADR-014: In-Panel Settings Entry

## Status

Accepted.

Date: 2026-06-22

## Context

Settings were reachable only via the menu bar or Cmd+,. New users had no in-panel affordance for configuration, weakening the "command center" self-containment of Route B.

## Decision

- Add a `gearshape` button (22×22 pt, `tertiaryLabelColor`, hover → `secondaryLabelColor`) at the right end of `LumaSearchBar`, left of the clear button.
- Click invokes the existing `SettingsWindowController` via `LauncherBridge.onOpenSettings` (same instance as menu bar / Cmd+,).
- Settings remain a separate 580×520 window — not embedded in the launcher panel.
- Button does not take keyboard focus (Tab skips).

## Consequences

Positive:

- Discoverable settings without hunting the menu bar.
- No change to settings lifecycle or SwiftUI Form implementation.

Negative:

- Slight reduction of search field trailing space; mitigated by 8 pt gap from clear button.

## Non-Goals

- In-panel embedded settings UI.
- Animated or scaled gear icon on hover.
