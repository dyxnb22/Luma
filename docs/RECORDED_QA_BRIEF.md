# Recorded QA Brief

This brief is the current handoff document for a full recorded Luma walkthrough. Use it together with:

- `docs/MANUAL_QA_CHECKLIST.md`
- `qa/SUMMARY.md`
- `qa/final/findings.md`
- `scripts/run_recorded_review.sh`
- `scripts/qa/run_full_smoke.sh`

## Goal

Run every currently shipped user-facing module and launcher flow at least once, while also judging visual polish, keyboard-first usability, error handling, and modern macOS fit.

## What Counts As In Scope

- Launcher hotkey, open/close, search, selection, Return, Tab, Esc
- Home list, search results, action panel, and same-panel detail navigation
- All active built-in modules
- Default-off modules after enabling them for the session
- Permission flows, empty states, and obvious recovery paths
- Visual and interaction quality, not just raw functional pass/fail

## Recommended Run Order

1. Start with a deterministic smoke baseline if needed:
   - `./scripts/run_recorded_review.sh`
   - or manually: `./scripts/qa/prep_smoke_env.sh` then `./scripts/qa/run_full_smoke.sh`
2. Then run a freeform recorded walkthrough:
   - launch
   - first-run or permission states
   - home list behavior (Open Apps only)
   - panel centering and module-prefix layout (`clip`, `note`, `tr`, … — no horizontal drift)
   - search and action panel
   - each module's primary path
   - at least one edge case per risky module
3. End with a short recap of findings while the recording is still running.

## Review Dimensions

- Functional correctness
- Usability and learnability
- Keyboard-only fluency
- Visual consistency and polish
- Performance feel
- Permission and recovery UX

## Required Findings Format

For each finding, capture:

- ID
- Severity: P0 / P1 / P2 / P3
- Type: defect / UX issue / visual polish / copy / performance
- Area or module
- Repro steps
- Expected behavior
- Actual behavior
- Recording timestamp or screenshot path
- Suggested direction

## Severity Guide

- P0: blocks core launcher usage or risks destructive/confusing behavior
- P1: major failure in a core flow or severe trust issue
- P2: meaningful friction, inconsistency, or moderate UX/visual problem
- P3: minor polish issue or edge-case improvement

## Notes For Cursor

- Prefer current Route C docs over historical Route A/Route B material.
- Authoritative freezes: `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md` (Open Apps home), `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md` (720×680 geometry, in-panel layout), `docs/specs/UX_BEHAVIOR_RULES.md` (navigation + shortcuts intent).
- Open launcher interaction gaps: `docs/qa/LAUNCHER_NAVIGATION_AUDIT.md` (temporary working doc — check before changing navigation, detail exit, or keyboard routing).
- Do not treat dashboard-card docs as current acceptance criteria.
- If behavior and documentation disagree, trust the current code plus `docs/ARCHITECTURE.md` and note the doc mismatch as a finding.
- Use scripted smoke to reduce setup drift; use freeform exploration to judge product quality.
