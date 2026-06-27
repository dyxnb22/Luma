# Luma New Modules Implementation Plan

## Understanding

Route C keeps Luma as a command-first single list. New modules must appear as trigger-driven result rows and may open same-panel detail views, but must not add dashboard cards or a sidebar. Module `handle()` paths stay memory-only and timeout-bounded; `warmup()` owns indexing/cache loading, and `perform()` owns side effects.

## Implementation Order

1. Quicklinks, estimated 2-3 hours.
   - Add models, JSON store, exact-trigger index, actions, detail management view, docs, help, tests.
   - Extend `SnippetVariableExpander` with `{{query}}` support through context rather than duplicating expansion logic.

2. Kill Process, estimated 1.5-2.5 hours.
   - Add running GUI app service, module filtering, quit/force/relaunch actions, confirmations, docs, help, tests.

3. Menu Bar Search, estimated 3-5 hours.
   - Add AX menu tree cache service, presser, fuzzy index, module, disabled bundle config, docs, tests for pure matching.

4. Browser Tabs Search, estimated 3-5 hours.
   - Add AppleScript adapters, timeout runner, actor cache, module, activate-tab action, docs, tests for parser logic.

5. Integration polish, estimated 1-2 hours.
   - Register module identifiers, built-ins, Settings visibility, FeatureCatalog detail entry, launcher detail routing, README and manual QA docs.

## Validation Plan

1. Build and unit tests, estimated 30-60 minutes.
   - `swift test`
   - `./scripts/build_app.sh`
   - Fix compile warnings introduced by this work.

2. Module smoke testing during implementation, estimated 30-60 minutes total.
   - Use launcher hotkey plus `scripts/qa/drive.sh`, not direct module calls.
   - Capture screenshots for each new trigger after the relevant module is implemented.

3. Full manual regression, minimum 3 rounds, estimated 1.5-3 hours per round.
   - Create `qa/round-N/screenshots/`, `findings.md`, and `summary.md`.
   - Run all required historical module trigger smoke tests plus the 4 new modules.
   - Run detail-view, action-panel, permission, edge-case, and performance checks.
   - Fix all P0/P1/P2 findings between rounds; P3 may be documented as known residual.

## Stop Criteria

Stop only after:

- Four modules are implemented and registered.
- `swift test` and app build pass.
- At least three QA round directories exist.
- The final full round has zero P0/P1/P2 findings.
- `qa/SUMMARY.md` records implementation scope, findings counts, known P3s, and performance data.
