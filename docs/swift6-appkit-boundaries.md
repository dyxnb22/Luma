# ADR: Swift 6 / AppKit Executor Boundaries

## Status

Accepted — 2026-07-06

## Background

Luma runs in Swift 6 mode (`swift-tools-version: 6.0`). AppKit Objective-C callbacks (`layout`, `draw`, `hitTest`, `keyDown`, `cancelOperation`, `performKeyEquivalent`, `mouseDown`, `viewDidChangeEffectiveAppearance`, `drawSelection`, `drawBackground`, `viewDidMoveToWindow`, `updateTrackingAreas`, `loadView`, `viewDidLoad`, `viewWillAppear`, `awakeFromNib`, target/action) are delivered on the main runloop or background queues without Swift's MainActor executor.

If a Swift `NSView` / `NSPanel` / `NSWindow` subclass is `@MainActor` (explicitly or inherited from `import AppKit` without `@preconcurrency`), the compiler inserts `_checkExpectedExecutor` in `@objc` thunks. When no Swift Task is on the MainActor executor, this triggers `EXC_BAD_ACCESS` or `_dispatch_assert_queue_fail`.

## Decision

1. Do **not** annotate `NSView` / `NSPanel` / `NSWindow` / `NSControl` / `NSTextField` / `NSTextView` / `NSTableRowView` / `NSViewController` subclasses **or project wrappers** (`LumaWindow`, `LauncherPanel`) with `@MainActor`.
2. AppKit subclass files must use `@preconcurrency import AppKit`.
3. All AppKit overrides must be `nonisolated override`.
4. When an override needs MainActor state, use `Task { @MainActor in ... }`. Do **not** use `MainActor.assumeIsolated`.
5. Do **not** use `Thread.isMainThread` as a proxy for MainActor.
6. `NotificationCenter` observers on MainActor-owned UI must be block-based with `Task { @MainActor in handler() }` — use `LumaNotificationCenter.observe`. Never `addObserver(self, selector:)` (including multiline formatting).
7. `NSViewController` lifecycle and `@objc` target/action: `nonisolated override func loadView()` (and other lifecycle overrides) with `Task { @MainActor in ... }` for state changes.
8. Carbon / C callbacks: `nonisolated` entry + `Task { @MainActor in ... }` (see `HotkeyController`).
9. Properties read from `nonisolated` overrides but written from MainActor use `nonisolated(unsafe)` with a comment.

## Show / hide generation guards

- `LauncherPanelVisibilitySession` — `shouldCompleteHide` / `shouldCompleteDeferredShow`; unit tests in `LauncherPanelVisibilitySessionTests`.
- `CancellationGeneration` — restore apply invalidation; unit tests in `CancellationGenerationTests`.
- `LauncherSnapshotApplyCoalescer.cancel()` on hide via `cancelPendingRestore()`.

## Cmd+Space routing

Only two paths: Carbon hotkey (`HotkeyController`) when panel hidden; `LauncherPanel.performKeyEquivalent` when panel visible (`guard isVisible`). Do not duplicate toggle handling in search field or list view.

## Exceptions

None. CI runs `scripts/scan_appkit_executor_risk.sh` (multiline `@MainActor` AppKit subclasses including `LumaWindow`/`LauncherPanel`, multiline selector observers, extended lifecycle overrides including `becomeKey`, `@preconcurrency import` on AppKit subclass files).

## Verification

- `scripts/scan_appkit_executor_risk.sh` exits 0.
- `swift test --filter AppKitExecutor` passes.
- Manual smoke checklist in `docs/QA.md` (Swift 6 section).
