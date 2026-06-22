# Dashboard Widget Polish Plan

Status: post Phase 0–7 backlog for Route B (ADR-007).
Date: 2026-06-22

## Completed in Phase 8–10 polish

- Sorted feature cards cached on construction.
- Crossfade and module-detail animation completion handlers guarded against stale state.
- `apply(snapshot:)` preserves the previously selected item across snapshot updates.
- `AppActivationTracker` coalesces persistence to a 1 s window with explicit `flush()`.
- `AppCoordinator` flushes the activation tracker on `NSApplication.willTerminateNotification`.
- `WidgetFeatureCard.animateScale` switched to `CATransaction`.
- Windows card trigger keyword normalized to `"win "`.
- Snapshot tests for `dashboardCoreCards()`.
- Manual QA checklist extended with Route B sections.

## Outstanding tracks

### Module detail real functionality

- Translate detail: inline source/target picker and copy button.
- Clipboard detail: scrollable history with pin/clear actions.
- Calculator detail: expression history and copy-result.
- Windows detail: real-time window list with focus action.

Each detail view should respect: keyboard focus stays in search bar by default; module detail can opt in to its own focus on activate().

### Settings interactivity

- Module toggles wired to `ConfigurationStore.setEnabledModules`.
- Hotkey rebinding control with conflict detection.
- Clipboard cap controls (max entries, age, body size).
- Reset-to-defaults button.

### Performance instrumentation

- `LatencyTelemetry` already DEBUG-only; add a rolling p95 ring buffer with periodic log emission.
- Track end-to-end keystroke → first paint, distinct from per-module time.
- Add a CI script that fails the build if the 1000-keystroke replay test regresses by > 20%.

### Visual polish

- Sidebar row hover state (subtle background, no animation).
- Result row hover state matching selection accent at lower alpha.
- Detail top bar Back button uses `chevron.backward` and respects RTL locales.

### Persistence hardening

- `AppActivationTracker` JSON corruption recovery (truncate and start fresh, never crash on load).
- `ClipboardHistoryStore` migration path when entry schema changes.

### Hot path discipline

- Profile `renderOpenApps()` actor hop cost during burst app switches.
- Cache `runningAppIcon` lookups by bundleID with NSCache.

## Out of scope (do not start without an ADR)

- Cloud sync.
- Plugin marketplace.
- Cross-platform support.
- Custom file index.
- Telemetry beyond local DEBUG logs.
