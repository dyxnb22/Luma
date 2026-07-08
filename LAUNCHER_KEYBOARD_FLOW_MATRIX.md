# Launcher Keyboard Flow Matrix (Phase 22.2)

**Date:** 2026-07-07  
**Purpose:** Contract table for real keyboard flows — initial state, inputs, expected axes, automation coverage, manual status.  
**Regression IDs:** [`USABILITY_REGRESSION_2026_07_07.md`](USABILITY_REGRESSION_2026_07_07.md)  
**Smoke runner:** `./scripts/qa/run_keyboard_flows.sh`

---

## Legend

| Column | Meaning |
|--------|---------|
| **Panel** | `visibilitySessionVisible`, AppKit `panelVisible` |
| **Search** | visible query, `isDetailModeActive`, editable |
| **Content** | `LauncherContentCoordinator.mode` |
| **Split** | `columnSplitActive` + `splitRightPane` (guide / detail / hidden) |
| **Focus** | first responder (search field, list, or detail subview) |

**Automation:** `swift test` filter, `run_p0_smokes.sh`, or `run_keyboard_flows.sh` flow id.

---

## Flow 1 — Cmd+Space open home

| | |
|---|---|
| **ID** | KF-01 |
| **Initial** | Panel hidden; empty query persisted or cleared |
| **Input** | `Cmd+Space` (Carbon hotkey — panel hidden) |
| **Panel** | visible; key window |
| **Search** | empty; editable; default placeholder |
| **Content** | `.home` |
| **Split** | column split; right = **guide** |
| **Focus** | search field |
| **Automation** | `HotkeyDoubleFire`, `LauncherShowEntry` — partial; **KF-01** keyboard smoke |
| **Manual fail** | No (when app running) |

---

## Flow 2 — Cmd+Space hide when visible

| | |
|---|---|
| **ID** | — (subset of KF-04) |
| **Initial** | Panel visible and focused |
| **Input** | `Cmd+Space` (panel `performKeyEquivalent` path) |
| **Panel** | hidden |
| **Search** | unchanged while hidden (detail may persist by design) |
| **Content** | unchanged on hide (no teardown) |
| **Split** | unchanged while hidden |
| **Focus** | n/a (panel ordered out) |
| **Automation** | `HotkeyDoubleFire`, `LauncherPanelExecutor` wiring — partial |
| **Manual fail** | UR-002 when desynced (Esc/hide confusion) |

Contract: [`LAUNCHER_SHOW_ENTRY_CONTRACT.md`](LAUNCHER_SHOW_ENTRY_CONTRACT.md) — visible ⌘Space hides only.

---

## Flow 3 — Esc at home closes panel

| | |
|---|---|
| **ID** | KF-01 (terminal step) |
| **Initial** | Panel visible; empty query; home mode |
| **Input** | `Esc` |
| **Panel** | hidden (fade) |
| **Search** | session saved |
| **Content** | `.home` |
| **Split** | n/a |
| **Focus** | n/a |
| **Automation** | `LauncherEscapePlanner` unit; **KF-01** smoke |
| **Manual fail** | No when home idle |

---

## Flow 4 — `n` / `n ` / `notes` → Notes detail or onboarding

| | |
|---|---|
| **ID** | KF-02 |
| **Initial** | Panel visible; home |
| **Input** | `n` + `Return` (bare open detail) **or** `n ` / `notes` + select row + `Return` |
| **Panel** | visible |
| **Search** | detail context; non-editable; suspended `n` or query |
| **Content** | `.detail(.notes)` **or** results with onboarding row |
| **Split** | column split; right = **detail** (if detail opens) |
| **Focus** | detail subview or search (non-editable) |
| **Automation** | **KF-02** via `drive.sh bare-open n` (`LUMA_QA` in-app bare-open hook) — asserts `showingDetail`, `currentDetailModuleID == luma.notes`, `splitRightPane == detail` (strict; `NotesProductionSmoke` covers router contract) |
| **Manual fail** | **Yes** — UR-001 (pre-22.5: placeholder + guide) |

**Contract:** Bare `n` opens detail when notes root configured (`NotesProductionSmoke.bareOpensDetail`).

---

## Flow 5 — Notes detail Esc

| | |
|---|---|
| **ID** | KF-02 (terminal step) |
| **Initial** | Notes detail open; suspended query empty or `n` |
| **Input** | `Esc` |
| **Panel** | stays visible |
| **Search** | editable; restored query if non-empty suspended query, else empty |
| **Content** | `.home` after exit |
| **Split** | column split; right = **guide** (crossfade if column split) |
| **Focus** | search field |
| **Automation** | `LauncherDetailExitPlanner`, `DetailTypingEscape` — unit; **KF-02** |
| **Manual fail** | **Yes** — UR-002 (Esc hid panel when `showingDetail` false) |

**Contract:** [`LauncherDetailExitPlanner`](Sources/LumaCore/Home/LauncherDetailExitPlanner.swift) — non-empty suspended query → restore to results; empty → `returnToHome(crossfadeToGuide: true)`.

---

## Flow 6 — Notes detail Cmd+Space

| | |
|---|---|
| **ID** | KF-04 (mid flow) |
| **Initial** | Notes detail open |
| **Input** | `Cmd+Space` |
| **Panel** | hidden |
| **Search** | detail mode may persist (hide does not tear down detail) |
| **Content** | `.detail(.notes)` may persist |
| **Split** | frozen while hidden |
| **Focus** | n/a |
| **Automation** | **KF-04** |
| **Manual fail** | Contributes to UR-003 on reopen |

---

## Flow 7 — Hide → reopen after Notes detail

| | |
|---|---|
| **ID** | KF-04 |
| **Initial** | Notes detail open |
| **Input** | `Cmd+Space` hide → `Cmd+Space` show |
| **Panel** | visible after reopen |
| **Search** | **must not** show Notes placeholder while right pane is guide only |
| **Content** | consistent: detail+detail pane **or** clean home |
| **Split** | no `isDetailModeActive && splitRightPane == guide` |
| **Focus** | search editable at home; detail focus if detail restored |
| **Automation** | **KF-04** — asserts snapshot invariants |
| **Manual fail** | **Yes** — UR-003 |

**Contract (Phase 22.5):** `reconcileLauncherStateAfterShow()` clears split-brain on reopen. **I5** fires only when the panel is **visible** with `isDetailModeActive && !showingDetail && splitRightPane == guide` — hide may preserve detail while the panel is hidden without tripping I5.

---

## Flow 8 — Clipboard detail Esc / Cmd+Space

| | |
|---|---|
| **ID** | KF-03 |
| **Initial** | Panel home |
| **Input** | `clip` + `Return` → detail; then `Esc` or `Cmd+Space` |
| **Panel** | visible after Esc; hidden after ⌘Space |
| **Search** | same contracts as Notes flows 5–6 |
| **Content** | `.detail(.clipboard)` when open |
| **Split** | right = **detail** |
| **Focus** | clipboard detail |
| **Automation** | `ClipboardProductionSmoke` (module); **KF-03** |
| **Manual fail** | Pending — UR-004 |

---

## Flow 9 — New query while in detail

| | |
|---|---|
| **ID** | — (unit only) |
| **Initial** | Module detail open; suspended prefix e.g. `n` |
| **Input** | Type new characters in search (when editable) or replace after cancel path |
| **Panel** | visible |
| **Search** | detail mode **cancelled**; new query visible |
| **Content** | `.results` |
| **Split** | single column (no split) |
| **Focus** | search field |
| **Automation** | `DetailTypingEscapeConsistency`, `handleTextChange` → `dismissDetailForNewQuery` |
| **Manual fail** | No in unit tests |

**Contract:** [`docs/QA.md`](docs/QA.md) — typing new query cancels detail; Esc does **not** restore suspended prefix after typing exit.

---

## Flow 10 — No result / disabled / permission rows

| | |
|---|---|
| **ID** | — |
| **Initial** | Panel visible |
| **Input** | Query yielding empty/disabled/permission row; `Return` / `Esc` |
| **Panel** | Esc from results → home; Esc from home → hide |
| **Search** | cleared on home transition |
| **Content** | `.results` then `.home` |
| **Split** | results: hidden; home: guide |
| **Focus** | list or search |
| **Automation** | `PermissionBanner`, `MVPModuleDiagnostic`, `LauncherGoldenReplay` — partial |
| **Manual fail** | Not reported in Phase 22.0 |

---

## Snapshot / invariant mapping

| Regression | Invariants | Keyboard flow |
|------------|------------|-----------------|
| UR-001 | I1, I3, I6 | KF-02 |
| UR-002 | I1 → escape | KF-02 Esc step |
| UR-003 | I5 (visible panel + detail mode + guide pane, no content detail) | KF-04 |
| UR-004 | I1, I3 | KF-03 |

---

*Phase 22.2 — documentation. Pass/fail columns updated by `run_keyboard_flows.sh` summary.*
