# Usability Regression — 2026-07-07 (Phase 22.0)

**Date:** 2026-07-07  
**Purpose:** Freeze real manual keyboard-flow failures before Phase 22 fixes. Each entry is a verifiable invariant breach.  
**RC status:** **No-Go** — see [`RC_BLOCKERS.md`](RC_BLOCKERS.md) § Phase 22.

**Build under test:** `./scripts/build_app.sh --no-restart` → signed `build/Luma.app`  
**Evidence tooling (Phase 22.1+):** `~/Library/Logs/Luma/launcher-state.json`, `launcher-state-violations.json`, `keyboard-flows/*`

---

## Summary matrix

| ID | Symptom | Invariant (planned) | Automation | Manual fail |
|----|---------|---------------------|------------|-------------|
| UR-001 | Notes detail placeholder + home/guide split mix | I1, I3 | `run_keyboard_flows.sh` KF-02 (`LUMA_QA` bare-open hook) | **Yes** |
| UR-002 | Esc dismisses panel instead of exiting detail | I1 → escape planner | `run_keyboard_flows.sh` KF-02 (`LUMA_QA` bare-open hook) | **Yes** |
| UR-003 | Cmd+Space reopen after Notes detail leaves mixed state | I5 (visible reopen split-brain only) | `run_keyboard_flows.sh` KF-04 | **Yes** |
| UR-004 | Clipboard detail Esc / hide path inconsistent | I1, I3 | `run_keyboard_flows.sh` KF-03 (results path; detail open via module smoke) | **Partial** |
| UR-005 | Menu bar Show → query → Esc stack wrong | — | `run_keyboard_flows.sh` KF-05 | **Pending** |

---

## UR-001 — Notes detail UI / content desync

| Field | Value |
|-------|-------|
| **Bundle** | Signed `build/Luma.app` from `./scripts/build_app.sh --no-restart` |
| **Preconditions** | Notes module enabled; notes root configured (`~/Library/Application Support/Luma/notes.json` has valid `root`); launcher hidden |
| **Input sequence** | `Cmd+Space` → type `n` → `Return` (bare open Notes detail) |
| **Expected** | Left: Open Apps column. Right: Notes detail (tree/map). Search field shows Notes detail context (non-editable placeholder). `contentCoordinator.mode == .detail(.notes)`. |
| **Actual** | Search field / hint bar shows Notes / detail context, but right column still shows **home command guide** (split pane). Open Apps visible on left — mixed home + detail chrome. |
| **Esc result** | Panel hides or wrong step (see UR-002) instead of detail exit |
| **Cmd+Space result** | See UR-003 |
| **Screenshot** | `~/Library/Logs/Luma/keyboard-flows/UR-001-notes-detail-mix.png` (captured by keyboard smoke on failure) |
| **Hypothesis** | `enterDetailContext()` runs before `contentCoordinator.present()` — `isDetailModeActive` true while `showingDetail` false (`LauncherDetailPresenter.swift`) |
| **Automation** | `scripts/qa/run_keyboard_flows.sh` flow KF-02 |

---

## UR-002 — Esc exits panel when detail context active

| Field | Value |
|-------|-------|
| **Bundle** | Same as UR-001 |
| **Preconditions** | UR-001 repro state (detail placeholder visible, guide pane on right) |
| **Input sequence** | `Esc` |
| **Expected** | `exitDetailFromChrome()` → crossfade to home guide OR restore suspended query; panel stays visible; search becomes editable |
| **Actual** | Panel **hides** (fade out) — `LauncherEscapePlanner` sees `showingDetail == false` and `queryTrimmedIsEmpty == true` → `.dismissPanel` |
| **Esc result** | Panel hidden |
| **Cmd+Space result** | Reopens to inconsistent state (UR-003) |
| **Screenshot** | `~/Library/Logs/Luma/keyboard-flows/UR-002-esc-hide.png` |
| **Hypothesis** | `handleEscape` uses `contentCoordinator.showingDetail` only, not `searchBar.isDetailModeActive` |
| **Automation** | `run_keyboard_flows.sh` KF-02 post-detail Esc assertion |

---

## UR-003 — Hide and reopen after Notes detail

| Field | Value |
|-------|-------|
| **Bundle** | Same as UR-001 |
| **Preconditions** | Notes detail opened via `n` + `Return` |
| **Input sequence** | `Cmd+Space` (hide while panel focused) → `Cmd+Space` (reopen) |
| **Expected** | Reopen: either restored Notes detail **or** clean empty-query home (Open Apps + guide). **Must not** combine Notes detail placeholder with guide-only right pane. |
| **Actual** | Notes detail placeholder in search bar persists; content area shows home/guide split — stale detail context without matching detail view |
| **Esc result** | Unpredictable (often hides panel) |
| **Cmd+Space result** | Mixed state persists across cycles |
| **Screenshot** | `~/Library/Logs/Luma/keyboard-flows/UR-003-reopen-mix.png` |
| **Hypothesis** | Hide preserves detail session (`prepareDetailForHide`); reopen does not reconcile `isDetailModeActive` vs `showingDetail` |
| **Automation** | `run_keyboard_flows.sh` KF-04 |

---

## UR-004 — Clipboard detail keyboard path

| Field | Value |
|-------|-------|
| **Bundle** | Signed `build/Luma.app` |
| **Preconditions** | Clipboard module enabled; launcher hidden |
| **Input sequence** | `Cmd+Space` → `clip` → `Return` → `Esc` |
| **Expected** | Clipboard detail in right column; Esc returns to home guide with editable search |
| **Actual** | **Pending verification** — same class of desync suspected |
| **Automation** | `run_keyboard_flows.sh` KF-03 |

---

## UR-005 — Menu bar Show entry

| Field | Value |
|-------|-------|
| **Bundle** | Signed `build/Luma.app` |
| **Preconditions** | Launcher hidden |
| **Input sequence** | Menu bar **Show Luma** → type `safari` → `Esc` |
| **Expected** | Panel shows; results for query; Esc clears to home; final Esc hides panel |
| **Actual** | **Pending verification** |
| **Automation** | `run_keyboard_flows.sh` KF-05 |

---

## Verification commands (post Phase 22.5)

```bash
./scripts/build_app.sh --no-restart
./scripts/qa/run_keyboard_flows.sh
# Manual: re-run UR-001–UR-003 sequences; confirm pass
```

---

*Phase 22.0 — documentation only. Fixes tracked in Phase 22.5.*
