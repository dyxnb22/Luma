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
- Auto Workflow status/list JSON parsing, detached PID parsing, shell argument splitting, and macOS app PATH handling
- Performance gates including keystroke replay and slow-module budgets

## Iteration Rule

For each meaningful product change:

1. Keep `swift test` green.
2. Add or adjust tests when behavior changes.
3. Run a smoke pass through `scripts/qa/run_full_smoke.sh` when launcher-facing behavior changes.
4. Run a focused recorded review when changes affect UX, permissions, navigation, or visual polish.
5. Log any defect or usability issue in `qa/` artifacts with reproduction steps and severity.

## Current Status

As of 2026-07-01:

- `swift test` passes locally with 533 tests.
- `qa/SUMMARY.md` records the last full recorded QA round with P0/P1/P2 = 0 open.
- The canonical scripted smoke entry point is `scripts/qa/run_full_smoke.sh`.
