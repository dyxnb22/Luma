# P2 Roadmap (Phase 14.3)

**Date:** 2026-07-07  
**Baseline:** `bc966c29`  
**Prerequisites:** P0 Exit **Go**, P1 Exit **Go** (`P1_EXIT_SUMMARY.md`)  
**Governance principle:** Small slices; doc-first where possible; **no big bang**.

---

## Overview

```
P2.1 Documentation / Manifest Hygiene     ← start here
    ↓
P2.2 Module Diagnostic Consistency        (P0 + Core P1 modules)
    ↓
P2.3 Module Lifecycle Contract Tests      (tests/linter; no ModuleHost rewrite)
    ↓
P2.4 Non-MVP AppKit Warn Cleanup          (Core P1 details; incremental)
    ↓
P2.5 QA Harness / Smoke Runner            (terminable gate + partial harness align)
```

**Global stop condition:** P0 gate failure (new `.ips`, smoke JSON failure, `swift test` red, hotkey/keystroke p95 breach) → revert active slice, P0 triage.

---

## P2.1 — Documentation / Manifest Hygiene

### Scope
- Fix `docs/PERMISSIONS.md` Default column (C-DEFAULT-004)
- Fix `WindowsModule` manifest `defaultEnabled: false` (metadata only)
- Add/clarify deferred vs parked vs registered status in module docs
- Record `LauncherSessionState` **test-only** decision (no wiring)

### Non-goals
- Enable any module
- Change runtime `defaultEnabled` except Windows manifest (unregistered)
- Resolve Todo Open Decision
- Touch `ModuleHost`, `QueryDispatcher`, Launcher code

### Files likely touched
- `docs/PERMISSIONS.md`
- `docs/MODULES.md` (registration status table)
- `Sources/LumaModules/Windows/WindowsModule.swift` (manifest only)
- `LAUNCHER_STATE_AUDIT.md` (decision stamp)
- `CONTRACTS.md` (mark C-DEFAULT-004 resolved when done)

### Tests
- `swift build` (no logic change expected)
- Optional: linter test that PERMISSIONS default column matches manifest grep script

### Stop conditions
- Any manifest change registers Windows in `ModuleRegistry`
- Any `defaultEnabled` flip on registered modules

### Acceptance
- [ ] PERMISSIONS Default matches `ModuleWarmupDefaults` + manifests
- [ ] Windows manifest `defaultEnabled: false`; still deferred
- [ ] MODULE_MATRIX / MODULES agree on parked list
- [ ] Session state: documented test-only for P2

**Estimated size:** 1 small PR

---

## P2.2 — Module Diagnostic Consistency

### Scope
- Publish single failure-behavior taxonomy (permission / warming / onboarding / empty / timeout)
- Align **Apps, Clipboard, Notes** first
- Then **Quicklinks, Snippets, Translate, Todo** (Core P1) if still default-on
- Parked modules: **record current behavior only**

### Non-goals
- New diagnostic enum rewrite
- QueryDispatcher ranking changes
- Parked module UX improvements
- New module features

### Files likely touched
- `docs/MODULES.md` or `docs/ENGINEERING.md` (taxonomy table)
- `Sources/LumaModules/Apps/`, `Clipboard/`, `Notes/` (row copy / diagnostic kind only)
- `Sources/LumaCore/Query/QueryDispatcher.swift` (only if generic `module.warming` synthesis needs tweak)
- `Tests/LumaModulesTests/` per-module diagnostic tests

### Tests
- `swift test --filter Apps`
- `swift test --filter Clipboard`
- `swift test --filter Notes`
- New: disabled module → diagnostic row; cold cache → warming row (per module)

### Stop conditions
- Search ranking or global search tier changes
- Silent empty for MVP targeted modules remains unfixed after slice

### Acceptance
- [ ] Taxonomy table exists and MVP modules reference it
- [ ] Notes onboarding row when root unset (C-FAIL-003)
- [ ] Clipboard permission row when AX denied (C-FAIL-001)
- [ ] Apps `app top` warming row documented and tested

**Estimated size:** 2–4 PRs (one module cluster per PR)

---

## P2.3 — Module Lifecycle Contract Tests

### Scope
- Contract tests for warmup / handle / perform / teardown on **Apps, Clipboard, Notes**
- `handle()` memory-only: extend `ModuleHandleContractTests` + optional static grep/linter script
- Document known exceptions (KillProcess, Wordbook, Windows) in `MODULE_MATRIX.md`

### Non-goals
- Rewrite `ModuleHost`
- Add KillProcess `teardown`
- Fix Windows `CGWindowListCopyWindowInfo` in `handle`
- Change async module actor contract

### Files likely touched
- `Tests/LumaModulesTests/ModuleHandleContractTests.swift`
- `scripts/` (optional `scan_handle_memory_only.sh`)
- `docs/MODULES.md` (lifecycle exceptions appendix)

### Tests
- `swift test --filter ModuleHandleContract`
- `swift test --filter Apps`
- `swift test --filter Clipboard`
- `swift test --filter Notes`

### Stop conditions
- ModuleHost API change required
- P0 smoke regression

### Acceptance
- [ ] Each P0 module: handle test passes (no await/network/disk in handle path)
- [ ] Teardown cancels refresh tasks (Apps running-app loop, Clipboard poll — verify existing)
- [ ] Exceptions table lists deferred modules explicitly

**Estimated size:** 2–3 PRs

---

## P2.4 — Non-MVP AppKit Warn Cleanup

### Scope
- Incremental `nonisolated @objc` + MainActor hop on **Core P1 detail views**: Snippets, Quicklinks, Translate, Todo
- Notes `NotesMindMapView` review (`.ips` frame history) — smoke only unless warn flagged
- **One file per PR**

### Non-goals
- Parked modules: Wordbook, Media, Secrets, Projects, CurrentProject bulk cleanup
- Scanner rule changes
- `docs/swift6-appkit-boundaries.md` rewrite

### Files likely touched
- `Sources/LumaApp/Launcher/SnippetsDetailView.swift`
- `Sources/LumaApp/Launcher/QuicklinksDetailView.swift`
- `Sources/LumaApp/Launcher/TranslateDetailView.swift`
- `Sources/LumaApp/Launcher/TodoDetailView.swift`

### Tests
- `bash scripts/scan_appkit_executor_risk.sh` (warn count ↓ for touched file)
- `swift build`
- `swift test --filter Launcher` (or module-specific)

### Stop conditions
- New `.ips` after detail view change
- MVP Clipboard/Notes regression

### Acceptance
- [ ] Touched file: zero scanner warns for that file
- [ ] Manual open/detail smoke for that module

**Estimated size:** 4+ PRs (one view each)

---

## P2.5 — QA Harness / Smoke Runner

### Scope
- `scripts/run_p0_smokes.sh`: terminable runner polling `~/Library/Logs/Luma/*-smoke.json`
- Optional: smoke hooks call `NSApp.terminate(nil)` after JSON write when `LUMA_QA_AUTO_EXIT=1`
- Partial `LauncherFlowHarness` align: `configureGlobalSearchModuleIDs` + production `CommandRegistry`

### Non-goals
- Full AppCoordinator E2E framework
- CI macOS runner requirement (script can be manual-first)
- EXPORT UI automation

### Files likely touched
- `scripts/run_p0_smokes.sh` (new)
- `Sources/LumaApp/Infrastructure/*ProductionSmoke.swift` (optional auto-exit)
- `Tests/LumaAppTests/Flow/LauncherFlowHarness.swift`
- `docs/QA.md` (P0 gate command update)

### Tests
- `./scripts/run_p0_smokes.sh` locally
- `swift test --filter LauncherFlowHarness`
- Full P0 gate sequence from `docs/QA.md`

### Stop conditions
- Smoke script hangs > 60s per module
- Harness align breaks existing golden tests without documented reason

### Acceptance
- [ ] Single script exits 0/1 with JSON artifacts
- [ ] Harness documents production parity or achieves router parity
- [ ] `docs/QA.md` references script

**Estimated size:** 2 PRs

---

## P2 exit criteria (target)

| Gate | Target |
|------|--------|
| Docs | C-DEFAULT-004 closed; Windows manifest fixed; parked manifest clear |
| Modules | P0 diagnostic + lifecycle contracts tested |
| Launcher | No P1 regression; session state decision recorded |
| QA | Terminable smoke script exists |
| P0 | Full gate green after each slice |

---

## Post-P2 backlog (not P2)

| Item | Track |
|------|-------|
| `LauncherSessionState` delete vs promote | P2.5+ / P3 |
| Todo default-on decision | User |
| Clipboard history scale / Notes polish | P2 late / product |
| Full harness parity | P3 |
| `docs/ENGINEERING.md` stale locations | P3.1 |
| Parked AppKit warns | P3 |

---

*Phase 14.3 — roadmap only; execution starts at P2.1.*
