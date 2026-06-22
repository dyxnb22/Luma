# Luma

Luma is a personal macOS launcher built for native speed, keyboard-first workflows, and local-first command execution.

The project is macOS-only: Swift 6, AppKit for the launcher surface, SwiftUI only where it helps in Settings/About, and in-process modules until the architecture has been dogfooded.

## Current Status

This repository is prepared as an engineering package and starter skeleton:

- Product and architecture docs live in `docs/`.
- Feature maintenance docs live in `Features/`.
- AI-assistant guardrails live in `.claude/`, `.cursor/`, and `.codex/`.
- Swift package targets are split into `LumaApp`, `LumaCore`, `LumaModules`, `LumaServices`, and `LumaInfrastructure`.
- Product route options are documented in [Product Route Options](docs/strategy/PRODUCT_ROUTE_OPTIONS.md). The currently accepted ADR favors launcher convergence, while the dashboard/widget route is preserved as an alternative implementation plan.
- Feature module stubs may exist for experiments, but v1 should optimize App Search, Window Focus, Clipboard History, Translate, Frecency Recents, and Calculator first.

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

```bash
./scripts/build_app.sh
open build/Luma.app
```

Building a `.app` bundle keeps a stable bundle identifier (`app.luma`), so macOS Accessibility permission survives rebuilds.

## Run on Login

After building the app bundle:

```bash
./scripts/install_launchd.sh   # install LaunchAgent
./scripts/uninstall_launchd.sh # remove LaunchAgent
```

The LaunchAgent points at `build/Luma.app/Contents/MacOS/Luma`. Re-run `build_app.sh` after code changes; no codesigning required for local use.

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
