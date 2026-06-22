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
- Snapshot tests for `dashboardCoreCards()`.
- Manual QA checklist extended with Route B sections.

## Completed in Phase 11–19

- Dormant modules (Notes, Wordbook, Secrets, WindowLayouts, Todo) removed from `BuiltInModules.makeAll()`.
- ClipboardDetailView: live history with copy / pin / delete.
- CalculatorDetailView: live expression evaluation.
- TranslateDetailView: live translation via system Shortcut with friendly error.
- WindowsDetailView: live window list with focus action.
- AX permission visible in Settings; in-panel banner on first run.
- Translation failures logged at error level.
- All deprecated APIs (`NSRunningApplication.activate()`, `NSApp.activate(ignoringOtherApps:)`) replaced.
- JSON quarantine on corruption for activation tracker and clipboard store.
- `build_app.sh` ad-hoc signs the bundle.
- About menu item with version + data location.
- NSAccessibility labels on widget cards, sidebar rows, result rows.
- Optional hotkey customization via Settings recorder.
- Rolling p95 latency window with real keystroke→first-paint timing.
- Documentation aligned to Route B as active route.

## Completed in Phase 20 — Core UX polish (Translate + Clipboard)

- Active dashboard reduced to **Translate** and **Clipboard** only (`FeatureCatalog.dashboardCoreCards()`).
- **Calculator** and **Windows** moved to `BuiltInModules.makeDeferred()` — source retained, excluded from warmup/default registration/active UI.
- Dashboard cards redesigned as wider liquid-glass widgets (24 pt corners, gradient + highlight, title/subtitle/status summary).
- Card status summaries: Translate shows target language + last status; Clipboard shows entry/pinned counts.
- Accessibility in-panel banner hidden while no active module requires AX.
- **TranslateDetailView** rewritten: language toolbar, dual input/output panels, action buttons, ⌘Return / ⌘C / Esc, search `tr`/`translate` opens detail with prefilled text.
- **ClipboardDetailView** rewritten: search, All/Text/Links/Pinned filters, keyboard navigation, pinned section labels, row hover actions, metadata (kind, time ago, chars, source app).
- `ClipboardEntry` extended with `detectedKind`, `sourceAppName`, `sourceBundleID`; token-based search.
- Tests updated for two-card dashboard, deferred modules, clipboard filters/pin persistence, translation user-facing errors.
- User translation input no longer logged.

## Out of scope (do not start without an ADR)

- Cloud sync.
- Plugin marketplace.
- Cross-platform support.
- Custom file index.
- Telemetry beyond local DEBUG logs.
- Re-enabling Calculator/Windows on the active dashboard without product review.
