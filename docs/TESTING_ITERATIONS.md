# Testing Iterations

## Current Test Strategy

Luma uses three complementary layers:

1. Unit and module tests via `swift test`
2. Tagged integration tests via `LUMA_INTEGRATION_TESTS=1 swift test --filter tag:integration`
3. Scripted manual smoke and recorded review passes through `scripts/qa/`

Primary references:

- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/specs/PERFORMANCE.md`
- `qa/SUMMARY.md`
- `qa/final/findings.md`
- `scripts/run_recorded_review.sh`

## Automated Coverage

The current suite covers launcher behavior and module logic including:

- Query routing, command hints, result ranking, and selection preservation
- App search fuzzy/pinyin matching
- Clipboard retention, filtering, pinning, image support, and destructive actions
- Notes indexing, capture, rename/delete/move flows, and detail support logic
- Wordbook scheduling, daily planning, manage flows, and review continuity
- Secrets vault metadata handling, lock/unlock behavior, and Keychain integration
- Quicklinks trigger expansion and variable substitution
- Menu Bar Search parser and cached query behavior
- Kill Process result generation and guarded actions
- Browser Tabs parsing, cache refresh behavior, and automation-path handling
- Performance gates including keystroke replay and slow-module budgets
- Launcher home split layout, guide/detail cross-fade hit-test policy, detail exit routing, and search detail-mode state (pure logic in `LumaCore`; AppKit wiring still manual)
- Documentation drift guard for deprecated spec phrases (`DocumentationDriftTests`)
- Action execution result mapping and usage/cache side-effect tests (`ActionExecutorTests`)

## Launcher UI Harness Gap

SwiftPM has no `LumaAppTests` target and AppKit UI automation is intentionally not wired in CI. The following remain **manual QA** (see `docs/MANUAL_QA_CHECKLIST.md`):

- Left Open Apps column keyboard focus and ↑↓ routing while split home is visible
- Guide ↔ detail cross-fade animation smoothness and z-order during transition
- Module detail subview shortcuts forwarded from focused controls
- Visual alignment of split divider and guide table after locale change

Pure planners under `Sources/LumaCore/Home/` cover state transitions that would otherwise require fragile view tests.

## Iteration Rule

For each meaningful product change:

1. Keep `swift test` green.
2. Add or adjust tests when behavior changes.
3. Run a smoke pass through `scripts/qa/run_full_smoke.sh` when launcher-facing behavior changes.
4. Run a focused recorded review when changes affect UX, permissions, navigation, or visual polish.
5. Log any defect or usability issue in `qa/` artifacts with reproduction steps and severity.

## Current Status

As of 2026-07-04:

- `swift test` passes locally with 576 tests.
- `qa/SUMMARY.md` records the last full recorded QA round with P0/P1/P2 = 0 open.
- The canonical scripted smoke entry point is `scripts/qa/run_full_smoke.sh`.
