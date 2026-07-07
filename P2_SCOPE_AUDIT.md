# P2 Scope Audit (Phase 14.1)

**Date:** 2026-07-07  
**Baseline commit:** `bc966c29` — *Refine launcher show-entry and session governance*  
**P0 exit:** `P0_EXIT_SUMMARY.md` — **Go**  
**P1 exit:** `P1_EXIT_SUMMARY.md` — **Go**  
**Scope:** Facts-only audit. **No Swift changes** in Phase 14.

**Sources:** `CONTRACTS.md`, `MODULE_MATRIX.md`, `MVP_SCOPE.md`, `REFACTOR_PLAN.md` §7, `LAUNCHER_STATE_AUDIT.md`, `P1_EXIT_SUMMARY.md`, `docs/PERMISSIONS.md`, `scripts/scan_appkit_executor_risk.sh`.

---

## P2 candidate inventory

| # | Item | Source Evidence | Current Risk | MVP Impact | Suggested Phase | Do Now? |
|---|------|-----------------|--------------|------------|-----------------|---------|
| 1 | **`LauncherSessionState` delete / promote / test-only** | `LAUNCHER_STATE_AUDIT.md`: 4/11 events wired; shadow diverges from `visibilitySession` + coordinator; `panelHideCompleted` never fires | Medium — misleading dual SoT if expanded; low if frozen | Low today (MVP works without full wiring) | **P2.1** decision doc; **P2.5+** migrate or delete | **No** — decide first, do not wire in P2.1 |
| 2 | **`docs/PERMISSIONS.md` Default column stale** | `CONTRACTS.md` C-DEFAULT-004; `docs/PERMISSIONS.md` marks Menu Items, Wordbook, Window Layouts, Media, Projects, Kill Process, Secrets, Workbench **on** vs manifests `defaultEnabled: false` | High — misleads contributors and support | None if doc-only fix | **P2.1** | **Yes** |
| 3 | **Windows deferred / `defaultEnabled: true` mismatch** | `MODULE_MATRIX.md` Windows row; `WindowsModule.swift:10` `defaultEnabled: true`; not in `ModuleRegistry.allBundles` | Medium — manifest lies about registration | None if metadata-only fix | **P2.1** | **Yes** (manifest flag only) |
| 4 | **Module warmup / handle / perform / teardown contract** | `CONTRACTS.md` C-MODULE-002/003; per-module exceptions (KillProcess teardown, Wordbook perform, Windows handle) | Medium — inconsistent lifecycle, hard to reason about hot path | Medium — P0 modules must be provably memory-only in `handle` | **P2.3** | **No** — tests first, not rewrite |
| 5 | **Diagnostic row / status consistency** | `CONTRACTS.md` C-FAIL-005; `MODULE_MATRIX.md` per-module ad-hoc cold/empty/onboarding strings | Medium — user sees inconsistent warming vs empty vs permission | Medium — P0 search UX clarity | **P2.2** | **No** — after P2.1 docs; P0 modules first |
| 6 | **Default-enabled module slimming (runtime)** | `MVP_SCOPE.md` D-012; Todo EventKit borderline; `REFACTOR_PLAN.md` P2.3 | Low if docs aligned; **high** if flipping switches | **High** if code changes defaults | **Defer / Needs User** | **No** — doc alignment only in P2.1 |
| 7 | **Deferred / parked module manifest clarity** | `REFACTOR_PLAN.md` §4 Decision #4; `MODULE_MATRIX.md` taxonomy; `BuiltInModules.makeDeferred()` | Medium — unclear what is registered vs deferred vs parked | Low if manifest table only | **P2.1** | **Yes** (docs + manifest metadata) |
| 8 | **`handle()` memory-only enforcement** | `CONTRACTS.md` C-HOT-001; deviation #9 "no generic enforcement"; `ModuleHandleContractTests` partial | Medium — Windows violation exists but deferred; P0 modules need proxy coverage | High for query hot path | **P2.3** | **No** — linter/test proxy, not ModuleHost rewrite |
| 9 | **Non-MVP AppKit `@objc` warn cleanup** | `scan_appkit_executor_risk.sh`: **78** warn-only; Clipboard MVP fixed in P1; Snippets/Wordbook/Translate/Todo/etc. remain | Low–medium — crash class risk on parked surfaces | Low for MVP path if scoped | **P2.4** | **No** — Core P1 details only, small steps |
| 10 | **Clipboard / Notes UX polish** | Phase 9 smokes pass; `MODULE_MATRIX.md` large clipboard history, Notes root onboarding | Low — polish not blocker | Low | **P2 late / P3** | **No** |
| 11 | **`LauncherFlowHarness` vs production wiring** | `CONTRACTS.md` C-TEST-004; empty `CommandRegistry`, no `configureGlobalSearchModuleIDs` | High — false confidence from harness-green | Medium — test signal quality | **P2.5** (align) / **P3** (full) | **No** |
| 12 | **Terminable `LUMA_QA_*` smoke runner** | `P1_EXIT_SUMMARY.md`: EXPORT non-terminating; Phase 9 smokes write JSON then app stays alive | Medium — manual gate friction | Medium — regression confidence | **P2.5** | **No** |

---

## P2 vs P3 boundary

| Keep in **P2** | Push to **P3** |
|----------------|----------------|
| Module manifest / permissions doc hygiene | `docs/ENGINEERING.md` location mismatches (C-UI-003) |
| P0/Core P1 diagnostic consistency table | Full `CONTRACTS.md` deviation sweep |
| Contract tests for MVP module lifecycle | Full harness ↔ production parity |
| Core P1 AppKit warn cleanup (incremental) | Parked-module AppKit cleanup |
| Terminable smoke runner | Test organization by MVP flow (`REFACTOR_PLAN` P3.2) |
| Windows manifest metadata | `LauncherContentMode` doc/code move decision execution |

---

## Explicitly out of scope (do not do in P2)

| Item | Why |
|------|-----|
| Rewrite `ModuleHost` / `QueryDispatcher` | P0 stability; REFACTOR_PLAN hard rule |
| Register / enable parked modules (Media, Secrets, Windows, …) | Decision Summary #4 |
| Fix `WindowsModule.handle` CGWindow call while deferred | C-HOT-001 violation accepted until re-entry |
| Wire all 11 `LauncherSessionState` events to production | Duplicates real owners; see audit |
| Change runtime `defaultEnabled` for Todo/Snippets/etc. without user decision | MVP_SCOPE Open Decisions |
| Clipboard 38MB history architecture / Notes mind-map feature work | Product scope, not governance |
| Full docs rewrite (`P3.1`) mixed into P2 | Sequencing rule |

---

## MVP impact summary

| Risk to P0 if P2 done wrong | Mitigation |
|----------------------------|------------|
| Default switch flip breaks fresh install | P2.1 doc-only; runtime defaults frozen |
| Diagnostic refactor changes search results | P2.2 scoped to row copy/taxonomy; no ranking |
| Lifecycle refactor touches hot path | P2.3 tests/linter only first |
| AppKit cleanup introduces regressions | P2.4 one file per PR; MVP modules first |

**Stop condition (all P2 slices):** Any P0 gate failure → revert slice and return to P0 triage per `P0_EXIT_SUMMARY.md`.

---

*Phase 14.1 — documentation only.*
