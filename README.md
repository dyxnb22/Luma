# Luma

Luma is a personal macOS launcher/workbench built for native speed, keyboard-first workflows, and modular feature cards.

The project is macOS-only: Swift 6, AppKit for the launcher and dashboard surfaces, SwiftUI only where it helps in Settings/About, and in-process modules until the architecture has been dogfooded.

## Current Status

This repository is prepared as an engineering package and starter skeleton:

- Product and architecture docs live in `docs/`.
- Feature maintenance docs live in `Features/`.
- AI-assistant guardrails live in `.claude/`, `.cursor/`, and `.codex/`.
- Swift package targets are split into `LumaApp`, `LumaCore`, `LumaModules`, `LumaServices`, and `LumaInfrastructure`.
- Feature module stubs are present for Translate, Clipboard History, Secrets Vault, Window Layouts, Notes Graph, Wordbook, Apps, Commands, Windows, and Calculator.

## Feature Direction

The first useful build should prove:

1. Command+Space toggles a pre-instantiated AppKit launcher panel.
2. Modules appear as rounded draggable cards with edit buttons.
3. Query input fans out to enabled modules through a timeout-protected dispatcher.
4. Clipboard history, translation, window layouts, secrets, notes, and word review each have isolated module boundaries.
5. Ranked results render quickly and actions run without blocking UI.
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
- [Manual QA Checklist](docs/MANUAL_QA_CHECKLIST.md)

## Non-Negotiables

- No Electron, Tauri, or WebView primary UI.
- No SwiftUI launcher panel.
- No plugin API in v1.
- No custom file index.
- No module may block the launcher hot path beyond its timeout.
- Default hotkey is Command+Space.
