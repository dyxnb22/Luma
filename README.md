# Luma

Luma is a personal macOS launcher built for native speed, keyboard-first workflows, and local-first command execution.

The project is macOS-only: Swift 6, AppKit for the launcher surface, SwiftUI only where it helps in Settings/About, and in-process modules until the architecture has been dogfooded.

## Current Status

Route B (Dashboard Widget single window) is the active product surface:

- Command+Space opens a pre-instantiated AppKit panel with a glass dashboard.
- **Active core cards:** Translate and Clipboard (two-column liquid-glass widgets).
- **Active modules:** Apps, Clipboard, Commands, Translate (plus open-apps sidebar).
- **Deferred from active UX** (source retained): Calculator, Windows, Notes, Wordbook, Secrets, Window Layouts, Todo.
- Translation uses Apple Translation / Shortcuts fallback — no network API.
- Clipboard history is local-first with secret filtering and pin/search support.

## Feature Direction

The first useful build should prove:

1. Command+Space toggles a pre-instantiated AppKit launcher panel.
2. Empty query shows real usage-based recents/frequents.
3. Non-empty query fans out to enabled modules through a timeout-protected dispatcher.
4. Ranked results render quickly and actions run without blocking UI.
5. The panel hides immediately after action dispatch.
6. Latency is measured from day one.

## Commands

```bash
swift build
swift test
```

## Build & Run

Install a stable local code-signing identity once:

```bash
./scripts/install_local_codesign_cert.sh
```

Then build and restart Luma:

```bash
./scripts/build_app.sh
```

`build_app.sh` stops any old Luma process, builds and signs the app, then opens the new build so Command+Space is registered again. Use `./scripts/build_app.sh --no-restart` only when you intentionally want to build without running Luma.

Building a `.app` bundle keeps a stable bundle identifier (`app.luma`). Signing with `Luma Local Development`, Apple Development, or Developer ID keeps Accessibility trust more stable across rebuilds than ad-hoc signing.

If Accessibility still appears enabled in System Settings but Luma shows the permission banner, reset the stale TCC record and re-enable Luma:

```bash
./scripts/repair_accessibility_permission.sh
```

## Run on Login

After building the app bundle:

```bash
./scripts/install_launchd.sh   # install LaunchAgent
./scripts/uninstall_launchd.sh # remove LaunchAgent
```

The LaunchAgent points at `build/Luma.app/Contents/MacOS/Luma` and restarts Luma after crashes while allowing normal Quit. Re-run `build_app.sh` after code changes.

## Key Documents

- [PRD](docs/PRD.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Engineering Package](docs/ENGINEERING_PACKAGE.md)
- [Roadmap](docs/ROADMAP.md)
- [Feature Index](Features/README.md)
- [Opus Decisions](docs/OPUS_DECISIONS.md)
- [Launcher Convergence Strategy](docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md)
- [Convergence Execution Plan](docs/strategy/CONVERGENCE_EXECUTION_PLAN.md)
- [Product Route Options](docs/strategy/PRODUCT_ROUTE_OPTIONS.md)
- [Dashboard Widget Strategy](docs/strategy/DASHBOARD_WIDGET_STRATEGY.md)
- [Dashboard Widget Cursor Plan](docs/strategy/DASHBOARD_WIDGET_CURSOR_PLAN.md)
- [Manual QA Checklist](docs/MANUAL_QA_CHECKLIST.md)

## Non-Negotiables

- No Electron, Tauri, or WebView primary UI.
- No SwiftUI launcher panel.
- No plugin API in v1.
- No custom file index.
- No module may block the launcher hot path beyond its timeout.
- No dashboard-first launcher panel.
- Default hotkey is Command+Space.
