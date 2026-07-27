# ADR-0007: Native workbench host

- Status: Accepted
- Date: 2026-07-26

## Context

The workbench is only as useful as it is reachable. Running `luma` inside Terminal.app means the
session is tied to a terminal window that gets buried, closed, or reused for something else, and
there is no way to summon it from wherever the keyboard focus currently is. ADR-0001 put a global
hotkey and a floating panel out of scope because the alternatives on the table at the time were a
second product UI (Tauri/Electron/native GUI). That reasoning still holds for a *product* UI; it
does not hold for a window that only hosts a PTY.

An earlier deleted Swift product tried to solve activation by owning the UI as well, and that is
exactly the shape this ADR refuses to restore.

## Decision

Luma may include a **thin native macOS PTY host** responsible for the window, terminal rendering,
global activation, singleton session, and lifecycle. All workbench UI, modules, commands, and
business state remain in the Rust TUI. The host must not become a second UI or module composition
root.

The host lives at `rust/native/luma-workbench/` (Swift Package Manager, AppKit, SwiftTerm pinned to
`1.15.0`) and is packaged locally as `Luma.app`:

```
Luma.app/Contents/MacOS/LumaWorkbench   # AppKit host (CFBundleExecutable)
Luma.app/Contents/MacOS/luma            # release Rust CLI, resolved as a sibling of the host
```

The host is not named `Luma` because APFS is case-insensitive by default, so `Luma` and `luma`
would collapse into a single file inside the bundle. The user-visible name comes from
`CFBundleName`. Its only child process is `luma tui`, started through a real PTY.

### The host owns

- app lifecycle (accessory app, explicit Quit / Cmd+Q, terminate-and-reap the child);
- window lifecycle (one persistent window; close button hides, never terminates);
- the global Command+Space hotkey (Carbon `RegisterEventHotKey`, no Accessibility permission);
- previous-frontmost-application capture and reactivation on hide;
- PTY child lifecycle, including restart on the next activation after the child exits;
- bounded graceful termination (SIGTERM handled by the TUI, three-second SIGKILL fallback);
- local combined host/child RSS sampling, peak tracking, and terminal-scrollback reduction when
  macOS reports memory pressure;
- app-bundle executable resolution and the child environment (`TERM`, `COLORTERM`, `PATH`);
- terminal font/background configuration and window-frame persistence.

Launch at Login is **not** implemented in this change. An accessory application has no visible
menu bar, and this change may not add a status item, so an `SMAppService` toggle would have had no
reachable control surface. Enabling it is a manual step (System Settings → General → Login Items)
until a later bounded change gives the host a control surface worth having.

### The host must not

- use SwiftUI;
- add a search box, results list, sidebar, preview, settings page, module picker, command palette,
  dashboard, or any native overlay above the terminal;
- read or write Luma module stores (Wordbook, Clipboard, Records, Projects, Timers, SSH,
  Proxy, Quicklinks, Snippets, Secrets, recipes) — it never touches LumaNext;
- initialize the Rust Engine, register modules, or duplicate Ratatui rendering/state;
- create a second theme or component system, multiple workbench windows, or tabs;
- add a background agent, daemon, Unix socket, HTTP server, or IPC protocol;
- change slash-prefixed command behavior;
- add release packaging, notarization, an updater, telemetry, or public distribution.

There is exactly one native content view: the SwiftTerm terminal filling the window.

## Consequences

- ADR-0001's "global hotkey / floating panel are out of scope" is amended: activation is now an
  allowed host concern, while the product shape (keyboard-first Rust TUI) is unchanged.
- The former menu-bar companion is removed. Its entry-point role is replaced by global activation;
  module status and actions remain in the TUI instead of being duplicated in another native UI
  ([ADR-0006](0006-native-menubar-companion.md)).
- The `luma` binary keeps working unchanged in any terminal emulator. The host is optional.
- The Swift host has no Rust dependency edges, so `scripts/check_architecture.sh` guards it by
  source inspection (no SwiftUI, no LumaNext access, no second status item) rather than by crate
  graph.
- Timers keep their current meaning, but hiding the window no longer pauses them: the child process
  stays alive while hidden. Quitting the host still terminates the TUI and its timers.
- Memory observations remain local unified-log entries, not telemetry or a diagnostics subsystem.
  SwiftTerm keeps its default 500-line scrollback normally and reduces it to 250/50 lines under
  warning/critical pressure.
- macOS permissions are per bundle. `com.luma.next.workbench` needs no Accessibility permission for
  the hotkey; any module that needs Accessibility (for example Windows focus) prompts under the
  host's own identity.

## Verification

Automated: `swift test --package-path native/luma-workbench` covers bundled-executable resolution,
PATH construction and deduplication, environment filtering, show/hide transitions, previous-app
tracking policy, child-exited → restart-on-next-show policy, hotkey debouncing, and integral grid
sizing. `scripts/check_architecture.sh` asserts the source-level boundary.

Manual: the workbench section of [`../MACOS_SMOKE.md`](../MACOS_SMOKE.md) covers cold launch, warm
activation, previous-app restoration, IME and CJK, copy/paste, resize, child processes, and child
restart. GUI activation behavior is not automated and must not be claimed as verified by tests.
