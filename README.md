# Luma

Personal keyboard-first launcher. **Product entry is the Rust CLI/TUI** under `rust/`.

The Swift AppKit tree (`Sources/`, `Tests/`, `Package.swift`) was removed after explicit approval on 2026-07-13. Recover it from tag `swift-legacy-final-20260713`.

## Status

Active surface: interactive CLI/TUI (`luma`). Legacy Swift AppKit launcher is retired from the default tree.

Migration / decommission docs:

- [SWIFT_DECOMMISSION_PLAN.md](SWIFT_DECOMMISSION_PLAN.md)
- [rust/docs/SWIFT_DELETION_READINESS.md](rust/docs/SWIFT_DELETION_READINESS.md)
- [rust/docs/DECOMMISSION_PROGRESS.md](rust/docs/DECOMMISSION_PROGRESS.md)
- [rust/docs/LEGACY_SWIFT.md](rust/docs/LEGACY_SWIFT.md)

## Commands (Rust)

```bash
cd rust
cargo run -p luma                 # interactive TUI
cargo run -p luma -- query "app" --json
cargo run -p luma -- doctor --json
cargo test --workspace --all-features
```

Data roots: `~/Library/Application Support/LumaNext/` and `~/Library/Logs/LumaNext/` (override with `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR` for tests).

Legacy Swift Application Support (`…/Luma`) is read-only via explicit `luma migrate …` importers.

## Rollback to Swift

```bash
git checkout swift-legacy-final-20260713
swift build
./scripts/build_app.sh   # if present on that tag
```

## Historical documents

Engineering / module / QA handbooks under `docs/` and root phase reports describe the former AppKit product and remain as behavior/migration reference unless separately retired.
