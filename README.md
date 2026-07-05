# Luma

Personal macOS launcher built for native speed, keyboard-first workflows, and local-first command execution.

Swift 6 · AppKit launcher · SwiftUI for Settings/About only · macOS 14+.

## Status

**Route C** is the active product surface: command-first unified list, with empty-query home showing Open Apps left and guide/detail right. Current phase: **integration and polish** — wire existing flows, tighten trust and keyboard behavior; no new surface area.

**For development:** start with [Engineering Handbook](docs/ENGINEERING.md).

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
| [Engineering Handbook](docs/ENGINEERING.md) | Product shape, architecture, launcher contract, performance, privacy, non-goals |
| [Module Handbook](docs/MODULES.md) | User-visible behavior for every built-in module |
| [Decision Log](docs/DECISIONS.md) | Compact active and historical ADR record |
| [QA, Testing, And Release](docs/QA.md) | Automated gates, manual smoke, recorded review, release checklist |

## Quick Review

```bash
./scripts/run_recorded_review.sh
```

Builds Luma, prepares the QA environment, runs the scripted smoke pass, then leaves you at the recorded walkthrough stage.
