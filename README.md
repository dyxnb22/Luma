# Luma

Luma is a personal macOS launcher built for native speed, keyboard-first workflows, and local-first command execution.

The project is macOS-only: Swift 6, AppKit for the launcher surface, SwiftUI only where it helps in Settings/About, and in-process modules until the architecture has been dogfooded.

## Current Status

**Route C** (Command-First Unified List, ADR-023) is the active product surface:

- Command+Space opens a pre-instantiated AppKit panel (~900×600 pt default; scales to ~840–940 × 580–700 pt by screen).
- **Empty query:** sectioned home list — Open Apps, Suggested.
- **Non-empty query:** flat ranked results (max 8 rows) from all enabled modules.
- **Module detail:** same-panel views entered via trigger keywords or contextual suggestions (not dashboard cards).
- **Built-in modules:** Apps, Clipboard, Commands, Notes, Todo, Translate, Wordbook, Snippets, Secrets, Media, Window Layouts, Projects, Quicklinks, Menu Bar Search, Kill Process, Browser Tabs, Auto Workflow.
- **Default-off modules in Settings:** Commands, Media, Browser Tabs, Auto Workflow.
- **Deferred from default registration** (source retained): Windows (window focus list).
- Translation uses Apple Translation / Shortcuts fallback — no network API.
- Clipboard history is local-first with secret filtering, image support, and pin/search.

## Current Focus

Luma is already broadly formed. The current phase is not feature sprawl; it is **making existing functionality feel fully connected and trustworthy**:

- wire cross-module flows cleanly
- improve permission and recovery UX
- remove dead or misleading docs and prompts
- tighten keyboard-first behavior, visual consistency, and empty states
- keep the hot path fast while polishing detail views

## Feature Direction

The launcher should prove:

1. Command+Space toggles a pre-instantiated AppKit launcher panel.
2. Empty query shows a keyboard-navigable home list (not a card grid).
3. Non-empty query fans out to enabled modules through a timeout-protected dispatcher.
4. Ranked results render quickly and actions run without blocking UI.
5. The panel hides immediately after action dispatch.
6. Latency is measured from day one (keystroke→paint and hotkey→home-rendered).

## Commands

```bash
swift build
swift test
./scripts/run_recorded_review.sh
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
- [ADR-023 Route C](docs/adr/023-command-first-unified-list.md)
- [ADR-031 Auto Workflow](docs/adr/031-autoworkflow-integration.md)
- [Engineering Package](docs/ENGINEERING_PACKAGE.md)
- [Opus Decisions](docs/OPUS_DECISIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Integration P0](docs/INTEGRATION_P0.md)
- [Feature Index](Features/README.md)
- [Performance Spec](docs/specs/PERFORMANCE.md)
- [Manual QA Checklist](docs/MANUAL_QA_CHECKLIST.md)
- [Recorded QA Brief](docs/RECORDED_QA_BRIEF.md)

## Quick Review

For a one-command recorded review pass:

```bash
./scripts/run_recorded_review.sh
```

It will build Luma, prepare the QA environment, run the scripted smoke pass, and then leave you at the recorded walkthrough stage with the current brief, checklist, and findings template.

## Non-Negotiables

- No Electron, Tauri, or WebView primary UI.
- No SwiftUI launcher panel.
- No plugin API in v1.
- No custom file index.
- No module may block the launcher hot path beyond its timeout.
- No dashboard-first launcher panel.
- Default hotkey is Command+Space.
