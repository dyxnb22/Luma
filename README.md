# Luma

Personal macOS launcher built for native speed, keyboard-first workflows, and local-first command execution.

Swift 6 · AppKit launcher · SwiftUI for Settings/About only · macOS 14+.

## Status

**Route C** (command-first unified list, [ADR-023](docs/adr/023-command-first-unified-list.md)) is the active product surface. Empty-query home is Open Apps left + guide/detail right ([ADR-032](docs/adr/032-home-split-command-guide.md)). Current phase: **integration and polish** — wire existing flows, tighten trust and keyboard behavior; no new surface area.

**For development:** start with [Engineering Package](docs/ENGINEERING_PACKAGE.md) (single entry point for reading order, frozen constraints, and module rules).

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

| Doc | Role |
| --- | --- |
| [Engineering Package](docs/ENGINEERING_PACKAGE.md) | **Primary dev entry** — reading order, conflict priority, module/workbench rules |
| [PRD](docs/PRD.md) | Product scope and success criteria |
| [Architecture](docs/ARCHITECTURE.md) | Runtime layers and module boundaries |
| [Manual QA Checklist](docs/MANUAL_QA_CHECKLIST.md) | Current regression / review checklist |
| [QA index](qa/README.md) | What to run vs historical run artifacts |
| [Feature Index](Features/README.md) | Per-module notes |

## Quick Review

```bash
./scripts/run_recorded_review.sh
```

Builds Luma, prepares the QA environment, runs the scripted smoke pass, then leaves you at the recorded walkthrough stage.
