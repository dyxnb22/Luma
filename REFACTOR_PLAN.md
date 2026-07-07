# REFACTOR_PLAN.md

## 1. Purpose

This is the **Phase 7** product of the Luma stabilization investigation. It is a roadmap, not a change list: it does not modify Swift source, tests, or scripts, and it does not stage or commit anything.

The roadmap exists to answer one question for every future PR: **"which of P0/P1/P2/P3 does this change serve, and does it move a specific, verifiable item forward?"** Its ordering is deliberate and non-negotiable:

1. **First, restore usable.** A signed app that stays running, a hotkey that works, and Apps/Clipboard/Notes/Settings/Diagnostics that function are worth more than any architectural improvement. Phase 0/5 evidence shows Luma currently fails this bar (no running process, three same-day `.ips`, hotkey p95 ≈ 8.3 s, diagnostics unreachable).
2. **Then, reduce core complexity.** Once the app runs, the riskiest complexity — `LauncherRootController`'s breadth, scattered UI-state ownership, `Task { @MainActor }` sprawl, and cache/refresh coupling to UI repaint — is what will keep causing regressions like the ones that produced the current state.
3. **Then, govern modules.** Once the core is stable and simpler, unify module lifecycle/diagnostic behavior and correct default-on/off mismatches, without expanding scope.
4. **Finally, align docs and tests** to the code and to `MVP_SCOPE.md`, and add a real (non-SwiftPM-only) smoke test as a release gate.

This roadmap explicitly is **not** a rewrite. It does not propose new architecture, does not pull `Media`/`Secrets`/`WindowLayouts`/`BrowserTabs`/`Windows`/complex `Workbench` back into the default path, and does not treat `swift test` green as sufficient evidence of anything beyond "SwiftPM compiled and ran unit-level logic."

## 2. Inputs

Phase 0-6 artifacts (read in full for this phase):

- `CURRENT_STATE.md` (Phase 0) — build/test/runtime/logs snapshot, `.ips` crash inventory, `latency-report.json` facts.
- `ARCHITECTURE_MAP.md` (Phase 1) — target graph, `AppCoordinator`/`LauncherRootController` ownership map, startup and query-to-render traces.
- `MODULE_MATRIX.md` (Phase 2) — per-module manifest/lifecycle/permission/diagnostic facts, code/docs mismatches.
- `PRODUCT_FLOWS.md` (Phase 3) — 16 numbered flows (F1-F16), Cross-Cutting State Owners table, per-flow test coverage and gaps.
- `CONTRACTS.md` (Phase 4) — 60 lettered/numbered contracts (C-LAYER-* through C-REVIEW-001), each with Current Known Deviations; the grading rubric this plan must align to.
- `USABILITY_TRIAGE.md` (Phase 5) — S-001 through S-026 symptom matrix with evidence levels, MVP blockers, diagnostic blockers, test coverage gaps.
- `MVP_SCOPE.md` (Phase 6) — MVP tiering (Core P0/P1, Recovery P0, Support P1), default-on/off target, Open Decisions, acceptance checklist.

Engineering/project docs (read in full):

- `docs/ENGINEERING.md` — layer table, Launcher Contract, Query/Module Contract, Performance Contract, non-goals.
- `docs/MODULES.md` — current D-012 default-on list (Apps, Clipboard, Snippets, Quicklinks, Todo, Translate, Notes), default-off list, per-module trigger/behavior facts. `MVP_SCOPE.md` further narrows the P0 main path to Apps/Clipboard/Notes plus Settings/Diagnostics recovery surfaces; Todo remains an Open Decision.
- `docs/PERMISSIONS.md` — permission matrix (confirmed stale Default column per C-DEFAULT-004; verified by direct read alongside `docs/MODULES.md` and manifests).
- `Package.swift` — target/dependency graph (`LumaCore` ← `LumaInfrastructure`/`LumaServices` ← `LumaModules` ← `LumaApp`; no dependency cycle).

Direct source/tooling verification performed in this phase (not inferred from docs alone):

- Ran `swift build` — build succeeds; the debug build was already cached, so no incremental warning output was reproduced in this phase, but Phase 0/5 both independently captured the same AppKit/MainActor warning class from a real build in `ClipboardDetailView.swift` and `LauncherListView.swift`.
- Ran `bash scripts/scan_appkit_executor_risk.sh` directly — **exits 0** (blocking checks 1-6 pass) but emits **89 warn-only findings**: `@objc` target/action methods inside `@MainActor` AppKit subclasses across 12 files (`ClipboardDetailView`, `SnippetsDetailView`, `WordbookDetailView`, `TranslateDetailView`, `TodoDetailView`, `MediaDetailView`, `SecretsDetailView`, `ProjectsDetailView`, `CurrentProjectDetailView`, `QuicklinksDetailView`, and others) that "should be nonisolated" per `docs/swift6-appkit-boundaries.md`. This is a directly-reproduced fact, not a doc citation.
- Grepped `Sources/LumaApp` for `Task { @MainActor` — **89 occurrences across 30 files**, confirming C-APPKIT-003's "not separately audited" deviation is a real, large surface, not a hypothetical.
- Confirmed `scripts/build_app.sh`, `scripts/measure_cold_start.sh`, `scripts/verify_manual_qa.sh`, `scripts/run_recorded_review.sh` exist in `scripts/`, so P0/P3 acceptance criteria that reference them are checking real, present tooling, not proposing new scripts.
- Confirmed `Package.swift` target graph matches C-LAYER-006's description exactly (no cycle, `LumaCoreTests` depends on `LumaModules` as the one recorded test-only edge).

No Swift source, test, script, or documentation file was modified during this phase. No files were staged or committed.

## 3. Decision Summary

1. Signed app runtime and hotkey availability outrank every other engineering priority; without them nothing else is verifiable in the field (S-025, S-002, MVP_SCOPE Core P0).
2. Diagnostics/Doctor is Recovery P0 and must be reachable on a default install outside Commands default-off gating; menu bar or Settings recovery entry is required, while `cmd doctor`/`cmd export-diagnostics` may remain mirrored command entries but cannot be the only path.
3. Apps, Clipboard, Notes, and Settings are the MVP main path (MVP_SCOPE Core P0); Snippets, Quicklinks, Translate are Core P1 candidates, not P0 gates; Todo remains conditional pending the EventKit Open Decision in `MVP_SCOPE.md` and is not a P0 gate until decided.
4. Media, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, and complex Workbench/Capture stay parked (source retained, not registered/warmed by default); this plan does not change that and does not schedule their re-entry.
5. Green `swift test` is necessary but not sufficient (S-026, C-TEST-004): `LauncherFlowHarness` diverges from `AppCoordinator.start()` production wiring (empty `CommandRegistry`, no `configureGlobalSearchModuleIDs`), so signed-app manual/scripted verification is required at every P0 gate.
6. `LumaModule.handle()` must stay memory-only (C-HOT-001); the one known violation (`WindowsModule.handle` calling `CGWindowListCopyWindowInfo`) is already deferred/unregistered and must stay that way — this plan does not fix it because fixing a parked module is out of MVP scope.
7. AppKit/MainActor boundary risk is treated as a P0-adjacent blocker, not a style issue: three same-day `.ips` crashes (`SIGSEGV`/`SIGABRT`) share the executor-boundary risk class in `docs/swift6-appkit-boundaries.md`, and a live scan in this phase found 89 warn-only `@objc`/`@MainActor` findings still present.
8. Documentation defaults must follow code and `MVP_SCOPE.md`, never the reverse: `docs/PERMISSIONS.md`'s Default column (marking Menu Items/Wordbook/WindowLayouts/Media/Projects/KillProcess/Secrets/Workbench "on") is confirmed stale against manifests and D-012, and must be corrected to match code, not used to justify re-expanding defaults.
9. `LauncherRootController` currently owns query routing, snapshot-apply enqueueing, detail presenter/lifecycle wiring, task registry, and three home providers (`ARCHITECTURE_MAP.md`); this plan narrows its responsibility list without rewriting it, and does so only after P0 runtime recovery.
10. Hot-path, AppKit/MainActor, diagnostics, permission, cache, and module-lifecycle work in this plan is graded against `CONTRACTS.md` contract IDs, not against ad-hoc taste; every task below cites the contracts it must not regress.
11. This plan does not reopen `MVP_SCOPE.md`'s remaining Open Decisions (Todo default-on/deferred, Projects minimal path, hotkey emergency ceiling). Phase 8.5 has already calibrated Translate/Snippets/Quicklinks as Core P1 candidates and Diagnostics reachability as outside Commands gating.
12. No task in this plan is "improve quality" or "reduce complexity" in the abstract — every task has a named file/type-level owner boundary, a reproducible acceptance check, and an explicit non-goal.

## 3.5 Phase 9 Entry Calibration

- Phase 9 P0 slices must not depend on Snippets, Quicklinks, Translate, Todo, Commands user scripts, or any parked/deferred module. Those surfaces may be verified opportunistically only after the P0 path is stable.
- Diagnostics/Doctor/export must be reachable outside Commands default-off gating. A menu bar or Settings recovery entry is required; mirrored Commands built-ins may remain, but they are not sufficient as the only path.
- Windows manifest/default mismatch is P2 hygiene. Windows remains deferred/unregistered and must not be selected as a Phase 9 P0 starting slice.

## 4. Phase Ordering

**P0 Runtime / Recovery → P1 Core Complexity → P2 Module Governance → P3 Docs / Tests.**

This order is forced by the evidence, not a stylistic preference:

- You cannot validate *any* other layer of work against a process that is not running. S-025 confirms no true `Luma.app/Contents/MacOS/Luma` process exists at snapshot time and three same-day `.ips` crashes are unexplained. Every P1/P2/P3 acceptance check in this plan that says "verify on signed app" is meaningless until P0.1 is closed.
- Diagnostics unreachability (S-020/S-021, C-DEFAULT-005, C-DIAG-001) means the team is currently debugging blind. Doing P1 complexity reduction without diagnostics reachable risks repeating the exact pattern the user flagged ("之前做的几轮审查，让项目越来越乱") — changes made without being able to see their effect.
- `LauncherRootController`/state-owner work (P1) touches the same code paths that produced three `.ips` crashes and the 8.3 s hotkey regression. Starting P1 before P0 stabilizes the runtime means every P1 change is unverifiable against real behavior and indistinguishable from noise in the crash/latency signal.
- P2 module governance (unifying lifecycle/diagnostic contracts, correcting default on/off docs) is safe only once the MVP core (Apps/Clipboard/Notes/Settings/Diagnostics) is itself stable; governing modules that sit behind an unstable core just produces more surface to re-break.
- P3 (docs/tests realignment, real smoke test) must come last because it documents and locks in the P0-P2 outcome; writing it earlier would either freeze stale facts or require rewriting it after every P0/P1/P2 change.

**Explicitly forbidden orderings:**

- No P2 module-governance work (contract unification across modules, default-on/off slimming) before P0 exits. Governing modules on top of a crashing/non-running app wastes the governance work.
- No new complexity added anywhere while `cmd doctor` / diagnostics export is unreachable (P0.8 open). Without diagnostics, every subsequent regression becomes another undiagnosable "basic functions do not work" report.
- No UI polish (visual, layout, animation) while the app cannot stay running or the hotkey cannot summon it within the MVP emergency ceiling (P0.1-P0.3 open).
- No expansion of deferred/parked modules (Media, Secrets, WindowLayouts, BrowserTabs, Windows, complex Workbench) while the Launcher main path (Apps/Clipboard/Notes/Settings) is unverified end-to-end on a signed app.

## 4.5 Phase 9 / P0 Exit — Achieved (2026-07-07)

**Status:** **Completed.** P0 MVP recovery slices 9.1–9.8 passed. Baseline commit: `889ebd35`.

**Evidence:** `P0_EXIT_SUMMARY.md`, `PHASE9_MVP_SMOKE_REPORT.md`, env-gated signed-app smokes (`LUMA_QA_EXPORT`, `LUMA_QA_APPS`, `LUMA_QA_CLIPBOARD`, `LUMA_QA_NOTES`, `LUMA_QA_SETTINGS`), `docs/QA.md` § P0 MVP Smoke Gate.

**P0 slices closed:**

| Slice | Item | Status |
|-------|------|--------|
| 9.1 | P0.1 Signed app / AppKit crash stop | ✅ |
| 9.2 | P0.2 Hotkey show/hide | ✅ (p95 ~28 ms; ≤ 1 s ceiling) |
| 9.3–9.3.1 | P0.8 Diagnostics / Doctor / export | ✅ (menu bar recovery; payload semantics) |
| 9.4 | P0.4 Apps search/open | ✅ |
| 9.5 | P0.5 Clipboard search/copy | ✅ |
| 9.6 | P0.6 Notes open/create | ✅ |
| 9.7 | P0.7 Settings open/save | ✅ |
| 9.8 | P0 gate integration | ✅ **Go** |

### Phase 11 — Launcher reduction (P1.1–P1.5 execution)

| Slice | Maps to | Status | Notes |
|-------|---------|--------|-------|
| 11.1 | — | ✅ | `LAUNCHER_STATE_OWNER_MAP.md` (docs only) |
| 11.2 | P1.1 | ✅ | RootController write gates; selection clamp fallback |
| 11.3 | P1.3 | ✅ | Detail exit + panel hide finalization helpers |
| 11.4 | P1.4 | ✅ | Clipboard MVP `@objc` executor boundary (partial) |
| 11.5 | P1.5 | ✅ | `LauncherHomeRefreshIntent`; visibility-session background gate |

**P1.2 (session state owner)** — **not started** in Phase 11; `LauncherSessionState` production wiring unchanged.

**Phase 11 P0 gate:** `swift build` + filtered tests ✅; no new `.ips`; signed-app smokes not re-run to completion (hooks are non-terminating; Phase 9.8 baseline stands). See `PHASE11_LAUNCHER_REDUCTION_REPORT.md`.

### Phase 12 — Session state audit & show entry governance

| Slice | Status | Notes |
|-------|--------|-------|
| 12.1 | ✅ | `LAUNCHER_SESSION_STATE_AUDIT.md` — facts only; recommend keep reducer test-only |
| 12.2 | ✅ | `LAUNCHER_SHOW_ENTRY_CONTRACT.md` |
| 12.3 | ✅ | `show(reason:)` + `showFromMenuBar()`; menu bar behavioral tests |
| 12.4 | ✅ | `LauncherListSelectionPreservePolicy` + `LauncherReturnActivationPolicy` + C-UI-004 tests |

See `PHASE12_SESSION_SHOW_GOVERNANCE_REPORT.md`.

### Phase 13 — P1 Exit / commit readiness

| Task | Status | Notes |
|------|--------|-------|
| 13.1 Review cleanup | ✅ | `appliesCarbonShowDebounce(reason:)` in `show(reason:)` |
| 13.2 Documentation pass | ✅ | `P1_EXIT_SUMMARY.md`; Phase 11/12 reports updated |
| 13.3 Full regression | ✅ | `swift test` 792/792 |
| 13.4 Signed app smokes | ✅ | Apps/Clipboard/Notes/Settings JSON smokes; EXPORT deferred |
| 13.5 P1 Exit | ✅ **Go** | See `P1_EXIT_SUMMARY.md` |

**P1 Exit:** Achieved — Launcher P1.1–P1.5 scope delivered across Phase 11–12; P1.2 session owner explicitly deferred to P2 per audit.

### Phase 14 — P2 planning (2026-07-07)

| Deliverable | Status |
|-------------|--------|
| `P2_SCOPE_AUDIT.md` | ✅ |
| `P2_DECISION_MATRIX.md` | ✅ |
| `P2_ROADMAP.md` | ✅ |

**P2 entry:** Planning complete (Phase 14). **Execution complete** (Phase 15, `8539007c`). **Exit Go** (Phase 16, `P2_EXIT_SUMMARY.md`). See `P2_ROADMAP.md` for slice detail.

**P2 exit:** **Achieved 2026-07-07** (`8539007c`, `P2_EXIT_SUMMARY.md`). Do not add new P2 slices without a new planning phase.

**P2 roadmap summary (execution order):**

| Phase | Focus | Status | Do not |
|-------|-------|--------|--------|
| **P2.1** | `docs/PERMISSIONS.md` defaults; Windows manifest metadata; parked/deferred manifest clarity; session state test-only stamp | ✅ `8539007c` | Enable modules; runtime default flips |
| **P2.2** | Diagnostic taxonomy; Apps/Clipboard/Notes (+ Core P1) row consistency | ✅ `8539007c` | Parked module polish; ranking changes |
| **P2.3** | Lifecycle contract tests; `handle()` memory-only linter/proxy | ✅ `8539007c` | ModuleHost rewrite |
| **P2.4** | Core P1 AppKit `@objc` cleanup (incremental) | ✅ `8539007c` | Bulk parked-module cleanup |
| **P2.5** | Terminable `LUMA_QA_*` runner; partial harness align | ✅ `8539007c` | Full AppCoordinator E2E |

**P3 remains separate:** `docs/ENGINEERING.md` location fixes, full harness parity, parked AppKit warns, full `CONTRACTS.md` deviation sweep — see `P2_SCOPE_AUDIT.md` P2 vs P3 boundary.

### Phase 15 — P2 execution (2026-07-07)

| Slice | Status | Commit |
|-------|--------|--------|
| P2.1 Documentation / manifest hygiene | ✅ | `8539007c` |
| P2.2 Module diagnostic consistency | ✅ | `8539007c` |
| P2.3 Lifecycle contract tests + handle scanner | ✅ | `8539007c` |
| P2.4 Core P1 AppKit cleanup | ✅ | `8539007c` |
| P2.5 Terminable smoke runner + harness align | ✅ | `8539007c` |

**Phase 15 report:** `PHASE15_P2_EXECUTION_REPORT.md`

### Phase 16 — P2 exit gate (2026-07-07)

| Gate | Result |
|------|--------|
| `swift test` | ✅ 801/801 |
| `scan_handle_memory_only.sh` | ✅ |
| `scan_appkit_executor_risk.sh` | ✅ (blocking) |
| `./scripts/run_p0_smokes.sh` | ✅ |
| New `.ips` during gate | ✅ none (30 min window) |

**P2 Exit:** **Go** — see `P2_EXIT_SUMMARY.md`. **P3 entry:** docs / test organization / release hardening only.

**P2 global stop:** Any P0 gate failure → revert slice, P0 triage (`P0_EXIT_SUMMARY.md`).

**P1 entry conditions (mandatory):**

1. **Run Phase 9.8 P0 MVP Smoke Gate** (`docs/QA.md`) before starting or merging any P1 work.
2. **Do not reopen parked modules** (Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, Workbench/Capture, Commands user scripts) as part of P1 unless they regress the P0 path.
3. **Do not expand MVP scope** to Core P1/conditional modules (Snippets, Quicklinks, Translate, Todo) without explicit user decision per `MVP_SCOPE.md`.
4. If gate fails (new `.ips`, hotkey p95 > 1 s, diagnostics unreachable, Apps/Clipboard/Notes/Settings path broken), **stop P1** and return to the matching P0 slice — do not refactor Launcher while P0 is red.

**P1 recommended order** (unchanged from §6):

1. P1.1 — `LauncherRootController` boundary
2. P1.2 — Launcher session state owner
3. P1.3 — Detail lifecycle
4. P1.4 — Task / MainActor boundary cleanup
5. P1.5 — Cache refresh vs UI repaint separation

## 5. P0 — 恢复可用

> **Note (2026-07-07):** P0 items P0.1–P0.8 were executed in Phase 9 and accepted at gate 9.8. Sections below remain as historical problem statements and acceptance criteria; do not re-open P0 work except on regression (see §4.5 P1 entry conditions).

### P0.1 Signed App Runtime / Crash Stop

**Problem / 现象** — No true `Luma.app/Contents/MacOS/Luma` process was found running (`pgrep -fl Luma` matched only Cursor Helper processes carrying the workspace label). Three same-day `.ips` crash reports exist for `app.luma` 0.1.0: two `EXC_BAD_ACCESS`/`SIGSEGV` and one `EXC_CRASH`/`SIGABRT`, all on faulting thread 0, with frames touching `@objc LauncherHomeGuidePane.tableView(_:shouldSelectRow:)` and `@objc NotesMindMapView.isFlipped.getter` — both in the AppKit executor-boundary risk class.

**Evidence / 证据来源** — `CURRENT_STATE.md` Runtime State and Logs And Diagnostics sections; `USABILITY_TRIAGE.md` S-025 (Confirmed, reproduced now); `CONTRACTS.md` C-APPKIT-001/002, C-ASYNC-001, C-TEST-003 and Current Known Deviations #15-16; direct scan in this phase (`scripts/scan_appkit_executor_risk.sh` exits 0 but reports 89 warn-only `@objc`/`@MainActor` findings across 12 detail-view files, confirming the risk class is still present in source).

**Owner Area** — `Sources/LumaApp/App/AppCoordinator.swift`, `LumaApp.swift`; AppKit view subclasses named in `.ips` frames (`LauncherHomeGuidePane`, `NotesMindMapView`); `docs/swift6-appkit-boundaries.md` compliance surface; `scripts/build_app.sh`, LaunchAgent per `README.md`.

**Desired Outcome** — A signed, built Luma process starts from a clean state, stays running, and does not crash during a scripted smoke pass covering startup, show, hide, and quit.

**Acceptance**
- `./scripts/build_app.sh` succeeds and produces `build/Luma.app/Contents/MacOS/Luma`.
- `pgrep -fl Luma` / `ps` show a process at `Luma.app/Contents/MacOS/Luma` (distinct from any Cursor Helper process carrying the workspace label), staying alive ≥ 10 minutes unattended.
- No new file appears under `~/Library/Logs/DiagnosticReports/Luma-*.ips` during a scripted smoke pass (startup → show → hide → show → quit, repeated ≥ 10×).
- `swift build` introduces no new compiler warnings versus the current baseline; the two known AppKit/MainActor warning sites (`ClipboardDetailView.swift`, `LauncherListView.swift`) are either fixed or explicitly logged as a non-blocking registered risk with a follow-up owner (do not silently accept new ones beyond that named set).
- `scripts/scan_appkit_executor_risk.sh` continues to exit 0 (blocking checks); any change to the 89 existing warn-only findings is tracked, not silently expanded.

**Risk** — Root-causing `EXC_BAD_ACCESS`/`EXC_CRASH` on AppKit callback frames may require touching code well beyond the two files with known compiler warnings; signing/entitlement/LaunchAgent issues (outside pure Swift) may also block this even after crashes are fixed.

**Dependencies** — None; this is the root blocker for every other item in this plan.

**Non-goals** — Do not rewrite `LauncherWindowController`/`AppCoordinator` startup sequencing as part of this task; do not attempt a general AppKit/SwiftUI migration; do not silence the AppKit executor scanner's warn-only findings by relaxing the script — the script's blocking checks must stay strict (per C-TEST-003).

---

### P0.2 Hotkey Show / Hide

**Problem / 现象** — `latency-report.json` records `hotkeyP95Milliseconds ≈ 8344.97` (~8.3 s) against a documented 50 ms p95 / 80 ms ceiling — roughly 100x over budget. Menu bar "Show" calls `windowController.show()` directly, bypassing `showFromCarbonHotkey()`'s hidden-only guard and the 120 ms Carbon-show debounce.

**Evidence / 证据来源** — `CURRENT_STATE.md` latency-report.json key fields; `USABILITY_TRIAGE.md` S-002 (Confirmed) and S-003 (Inferred Risk); `CONTRACTS.md` C-HOT-002, C-ASYNC-003, C-UI-001/006, Current Known Deviations #12, #14; `MVP_SCOPE.md` Hotkey Show/Hide acceptance section (1 s emergency ceiling vs 50/80 ms long-term contract).

**Owner Area** — `Sources/LumaApp/App/Hotkey/HotkeyController.swift`, `Sources/LumaApp/Launcher/LauncherWindowController.swift` (`show()`, `showFromCarbonHotkey()`, `toggle()`), `Sources/LumaApp/App/MenuBarController.swift`, `HomeLatencyTracker`; `HotkeyConfig.save()` no-op (Settings interaction, see P0.7).

**Desired Outcome** — Cmd+Space (or the current default hotkey) reliably shows/hides the panel within the MVP emergency ceiling, and menu bar Show works as a consistent fallback.

**Acceptance**
- Cold-started signed app: Cmd+Space shows the panel within the short-term MVP ceiling of ≤ 1 s; long-term contract target (50 ms p95 / 80 ms ceiling) remains the documented goal in `docs/ENGINEERING.md` and is not silently lowered.
- 50 consecutive show/hide/show cycles via Cmd+Space produce no stuck-visible, transparent, duplicate, or unfocused panel state, and no new `.ips`.
- Menu bar "Show" opens the panel when the hotkey path has not fired, and rapid Show + Cmd+Space sequencing does not leave the panel in an inconsistent visibility state (per C-UI-001/006 — this does not require removing the bypass, only proving it is safe or fixing it if it is not).
- `HotkeyConfig.save()` no-op's effect on Settings is explicitly documented in this pass (UI must not claim a hotkey edit persisted when it did not) — cross-reference P0.7.
- A new `latency-report.json` run after the fix shows `hotkeyP95Milliseconds` materially reduced from the ~8.3 s baseline, with the delta explained (root cause named), not just masked by a different code path.

**Risk** — The 8.3 s regression's root cause is undiagnosed (Phase 0-6 explicitly did not diagnose it); it may be in Open Apps refresh, home rendering, or elsewhere in the show path (C-ASYNC-003), and fixing it may require touching `OpenAppsHomeProvider`/`LauncherHomeCoordinator` behavior that P0.1's crash fixes also touch — sequence carefully to avoid conflating regressions.

**Dependencies** — P0.1 (a process must stay running long enough to measure hotkey latency at all).

**Non-goals** — Do not redesign the Carbon/menu-bar dual-path architecture; do not remove the menu bar bypass unless it is proven unsafe; do not chase the full 50/80 ms contract ceiling as a P0 gate — the 1 s emergency ceiling is the P0 bar per `MVP_SCOPE.md` Open Decision 6.

---

### P0.3 Launcher Input Responsiveness

**Problem / 现象** — Keystroke p95 is currently within budget (19.13 ms vs 30/60 ms target/ceiling), but this is measured only via `LauncherFlowHarness`, which diverges from production wiring (empty `CommandRegistry`, no `configureGlobalSearchModuleIDs`) and does not exercise `AppCoordinator.start()`. There is no confirmation that typing is not blocked by disk/AX/AppleScript/EventKit/process work on a real running app, and the session-state owner for query/loading/empty/result states is split across `LauncherViewModel`, `LauncherRootController`, and `LauncherContentCoordinator`.

**Evidence / 证据来源** — `CURRENT_STATE.md` latency-report.json (`keystrokeP95Milliseconds=19.13`); `CONTRACTS.md` C-HOT-001/002 and Current Known Deviations #9; `USABILITY_TRIAGE.md` S-006/S-007 (Needs user confirmation / Not reproduced); `ARCHITECTURE_MAP.md` Launcher Query-To-Render Flow section (`LumaSearchBar` → `LauncherRootController.handleTextChange` → `LauncherViewModel.queryChanged` → `QueryDispatcher`).

**Owner Area** — `Sources/LumaApp/Launcher/LumaSearchBar.swift`, `LauncherRootController.swift` (`handleTextChange`), `Sources/LumaCore/Query/QueryDispatcher.swift`; per-module `handle()` implementations (must stay memory-only per C-HOT-001).

**Acceptance**
- On the signed, running app (not the harness), typing a representative query string produces no visible UI freeze; a scripted or manual keystroke-to-paint measurement is captured from the real app, not only `LauncherFlowHarness`.
- `keystrokeP95Milliseconds` in a fresh `latency-report.json` run stays within the documented budget (30 ms p95 / 60 ms ceiling) on the signed app.
- Empty query shows home immediately (no blocking on Open Apps rebuild, per C-ASYNC-003).
- Every query state (loading/empty/result/diagnostic) is visibly distinct — no silent disappearance of rows (per C-HOT-004/005, C-FAIL-002).

**Desired Outcome** — Confirmed, signed-app-verified input responsiveness that matches the harness's already-passing numbers, closing the S-026 gap between test-green and production-real for this specific flow.

**Risk** — If real-app keystroke latency diverges materially from the harness's 19.13 ms, this task's scope could balloon into a full `QueryDispatcher` performance investigation; timebox the investigation and escalate to P1.2 (session state owner) if the root cause is state-management overhead rather than hot-path violations.

**Dependencies** — P0.1 (signed app must run), P0.2 (panel must be visible to type into).

**Non-goals** — Do not rewrite `QueryDispatcher` tiering or ranking; do not add new performance instrumentation beyond confirming existing `latency-report.json`/`LauncherPerfCounters` numbers on the real app; do not fix `LauncherFlowHarness`/production divergence here (that is P3.2).

---

### P0.4 App Search / Open

**Problem / 现象** — Apps is the MVP P0 main path (launch/focus apps, `app top` memory view), but no live-app confirmation exists that `app safari` returns and activates Safari, or that `app top` shows a warming-then-real-rows sequence on a signed app. `launcherFlowHarnessReplaysQuery` failed in Phase 0 and passed in later phases — a flip that itself is unexplained.

**Evidence / 证据来源** — `MVP_SCOPE.md` App Search/Open acceptance criteria; `USABILITY_TRIAGE.md` S-004/S-006/S-010 (User-Reported, Needs user confirmation); `CURRENT_STATE.md` Test Results table (harness flip); `CONTRACTS.md` C-HOT-001/003/004, C-FAIL-002, C-MODULE-002.

**Owner Area** — `Sources/LumaModules/Apps/` (AppsModule and its store/index), `Sources/LumaCore/Query/QueryDispatcher.swift` (global fan-out — Apps is a contributing module per `GlobalSearchTiers`), `ActionExecutor` (launch/focus perform path).

**Desired Outcome** — Confirmed, signed-app, main-path proof that Apps search-and-open works end to end, closing S-004/S-006/S-010 for the single most important MVP module.

**Acceptance**
- On the signed app, `app safari` (or an installed equivalent) returns a result and Return launches/activates it.
- `app top` shows a warming row when the memory cache is cold, followed by real rows (C-FAIL-002) — verified live, not only via `ModuleColdCacheTests`.
- Bare `app` shows Apps guide/rows, never a blank panel.
- No AX banner appears on plain home / ordinary app search when Accessibility is denied (D-010) — verified live.
- Failure paths (no permission, cold cache) produce a diagnostic row/status, never silent nothing.
- Existing automated coverage (`AppsModuleTests`, `AppsModuleTopQueryPerformanceTests`, `AppsMemoryTopSWRTests`, `simulatedUserTouchesEveryRequestedFeature`, `appsModuleTopTargetedQueryStaysUnderBudget`) continues to pass, and at least one of these is re-run against the signed app path (not harness-only) as part of the smoke checklist.

**Risk** — Apps is a contributing module in global search fan-out (C-HOT-003); any fix here has blast radius into global search ranking/timing for Clipboard/Quicklinks too — keep changes scoped to Apps-specific code paths.

**Dependencies** — P0.1-P0.3 (app must run, be summonable, and accept input).

**Non-goals** — Do not introduce new ranking logic; do not add app metadata beyond what warmup already caches; do not expand Apps' global-search fan-out behavior.

---

### P0.5 Clipboard Basic Search / Copy

**Problem / 现象** — Clipboard is MVP P0, but `clipboard-history.json` is ~38 MB with a sibling `.corrupt-1782301078.bak`, raising risk that detail open or paste-directly could be slow or blocked. `ClipboardDetailView.swift` is one of the two files with confirmed AppKit/MainActor compiler warnings.

**Evidence / 证据来源** — `USABILITY_TRIAGE.md` S-016 (User-Reported, Needs user confirmation) and Runtime and logs section (37.9 MB history + corrupt backup, confirmed by direct `ls`); `CONTRACTS.md` C-HOT-001, C-FAIL-001, C-PERSIST-001, C-CACHE-002; `MVP_SCOPE.md` Clipboard Search/Copy acceptance criteria; direct scan in this phase confirming `ClipboardDetailView.swift` still has 19 `@objc`-in-`@MainActor` warn-only findings.

**Owner Area** — `Sources/LumaModules/Clipboard/ClipboardHistoryStore.swift`, `Sources/LumaApp/Launcher/ClipboardDetailView.swift`.

**Desired Outcome** — Confirmed, signed-app proof that Clipboard's core loop (search, select, copy) works without being blocked by history size or corruption artifacts, without attempting a full history redesign.

**Acceptance**
- `clip` opens Clipboard detail on the signed app without a perceptible stall attributable to the 38 MB history file.
- Search (≥3 chars per global cap, or in-detail search) returns in-memory matches; Return copies the selected entry to the pasteboard.
- Paste-directly shows `permissionRequired(.accessibility)` guidance when AX is denied — never a silent success (C-FAIL-004).
- The existing `.corrupt-*.bak` file does not block detail open or search (verify, do not necessarily clean it up — full history cleanup is explicitly out of scope per `MVP_SCOPE.md` Risk Register).
- `ClipboardDetailView.swift`'s AppKit/MainActor warn-only findings from the scanner are resolved or explicitly registered as non-blocking (tie to P0.1's warning-baseline acceptance).

**Risk** — Large-history performance issues may be structural (O(n) haystack rebuilds, unbounded in-memory store) rather than a quick fix; resist the urge to redesign `ClipboardHistoryStore` storage format under P0 time pressure.

**Dependencies** — P0.1-P0.3.

**Non-goals** — No full clipboard history cleanup/migration, no new corruption-recovery UI, no `ClipboardDetailView` feature work beyond search/copy/paste-directly and closing its AppKit warnings.

---

### P0.6 Notes Basic Open / Create

**Problem / 现象** — Notes is MVP P0 and default-on, but the most recent `.ips` crash (2026-07-06 11:56:51) has a faulting frame at `@objc NotesMindMapView.isFlipped.getter` — an AppKit executor-boundary crash directly inside a Notes detail view.

**Evidence / 证据来源** — `USABILITY_TRIAGE.md` S-017 (User-Reported, P0, "crash-adjacent evidence") and the `.ips` summary in Logs And Diagnostics; `MVP_SCOPE.md` Notes Open/Create acceptance criteria and Known Blockers; `CONTRACTS.md` C-FAIL-003, C-PERSIST-001, C-DETAIL-003, C-APPKIT-001, D-016.

**Owner Area** — `Sources/LumaApp/Launcher/NotesMindMapView.swift`, `NotesDetailView.swift` (23 `Task { @MainActor` occurrences per direct grep — the highest count of any single file in the app, and worth a dedicated look here since it is also the crash site), `Sources/LumaModules/Notes/`.

**Desired Outcome** — Confirmed, signed-app proof that Notes' create/open/onboarding loop works and does not reproduce the `NotesMindMapView.isFlipped` crash class.

**Acceptance**
- Bare `n` opens Notes detail, or — if no root is configured — shows the "Choose a Notes root folder" onboarding row (C-FAIL-003), never a blank panel.
- `n new` creates a note under the configured root and opens it via `openLocalFileURL` after path containment validation (D-016).
- Switching Tree/Map view in Notes detail (the `NotesMindMapView` surface implicated in the `.ips`) does not crash across a repeated manual/scripted pass.
- No new `.ips` involving `NotesMindMapView` or other Notes AppKit overrides during the smoke pass.
- Esc from Notes detail returns to home/results with the search field editable again (per launcher-navigation workspace rule).

**Risk** — `NotesDetailView.swift`'s 23 `Task { @MainActor }` sites make this the single highest-density file for the async/AppKit boundary risk in the app; a narrow fix targeting only `NotesMindMapView.isFlipped` may not be sufficient if the crash's real cause is elsewhere in the same file's Task/MainActor interplay.

**Dependencies** — P0.1 (shares the same crash risk class), P0.1-P0.3.

**Non-goals** — No Workbench/Capture integration; no full outline/mind-map feature work; no attempt to make `NotesDetailView.swift`'s async patterns exemplary — only enough to stop the specific crash class and confirm the P0 acceptance bar.

---

### P0.7 Settings Open / Save

**Problem / 现象** — Settings is the designated recovery surface, but `HotkeyConfig.save()` is a confirmed no-op by code fact, and Commands being default-off may cut off the `settings` command entry (menu bar Settings must remain independent of that). No full Settings open/save/restart-persistence test was confirmed to run in any prior phase.

**Evidence / 证据来源** — `USABILITY_TRIAGE.md` S-019 (Inferred Risk, P0, "No full Settings open/save test run"); `MVP_SCOPE.md` Settings Open/Save acceptance criteria and mismatch list (`HotkeyConfig.save()` no-op); `CONTRACTS.md` C-FAIL-003, C-PERSIST-001, C-DEFAULT-005.

**Owner Area** — `Sources/LumaApp/Settings/SettingsWindowController.swift`, `SettingsSwiftUIView.swift`; `Sources/LumaCore` `ConfigurationStore`/`HotkeyConfig`; `Sources/LumaApp/App/MenuBarController.swift` (menu bar Settings entry).

**Desired Outcome** — Settings reliably opens from the menu bar independent of the Commands module state, and at least one non-destructive setting (e.g. Notes root, enabled modules) persists across restart; the hotkey-save no-op no longer misleads the user.

**Acceptance**
- Menu bar Settings opens the Settings window on a default install, independent of whether Commands is enabled.
- Saving a non-destructive setting (Notes root, enabled modules, Clipboard retention) persists across a full app restart (`./scripts/build_app.sh` restart cycle).
- If the Settings UI exposes a hotkey field, it either persists correctly or is visibly marked non-functional — it must not silently accept input that `HotkeyConfig.save()` then discards (C-FAIL-004 spirit: no false success).
- Config read failure or corruption is visible somewhere reachable from Settings or `cmd doctor` (tie to P0.8), not a silent fallback.

**Risk** — Fixing `HotkeyConfig.save()` to actually persist may interact with the Carbon hotkey registration path touched in P0.2; sequence after P0.2's hotkey work is stable, or scope this task to "make the UI honest" without necessarily making the hotkey re-registrable if that proves riskier.

**Dependencies** — P0.1-P0.3; interacts with P0.2 (hotkey) and P0.8 (doctor visibility of config corruption).

**Non-goals** — Do not build a full hotkey-remapping feature; do not redesign the Settings UI; do not make every setting persist — only the non-destructive ones named in `MVP_SCOPE.md`.

---

### P0.8 Diagnostics / Doctor / Export

**Problem / 现象** — `cmd doctor` and `cmd export-diagnostics` are unreachable on a fresh install because they live behind the default-off Commands module, with no documented alternative entry. `~/Library/Logs/Luma/diagnostics.json` is confirmed missing. The production `DiagnosticsExport` call site populates only `latencyP95` + `CrashLogBuffer` breadcrumbs — `platform`/`modules`/`permissions`/`recentErrors` are `nil`/empty despite being documented as populated. `crash-log.txt` actually lives at `~/Library/Application Support/Luma/`, not `~/Library/Logs/Luma/` as docs imply. `CrashLogBuffer.persist()` swallows write failures via `try?`.

**Evidence / 证据来源** — `USABILITY_TRIAGE.md` S-020, S-021 (Confirmed, P0, blocks MVP), S-022, S-024; `CONTRACTS.md` C-DIAG-001/002/003/004, C-DEFAULT-005, C-PERSIST-002, and Current Known Deviations #3-8; `MVP_SCOPE.md` Diagnostics / Doctor / Export section (Recovery P0, target ≠ current fact) and Phase 8.5 calibration decision (recovery entry outside Commands default-off gating).

**Owner Area** — `Sources/LumaModules/Commands/CommandsModule.swift` (doctor/export-diagnostics built-ins), `Sources/LumaApp/Infrastructure/AppHostService.swift` (`exportDiagnostics` call site), `Sources/LumaCore/Util/DiagnosticsExport.swift`, `Sources/LumaInfrastructure/CrashLogBuffer.swift`, `Sources/LumaCore/Persistence/ConfigCorruptionRegistry.swift`.

**Desired Outcome** — A default install can always reach doctor/export-diagnostics through a menu bar or Settings recovery entry outside Commands default-off gating. Commands built-ins may mirror the same doctor/export behavior, but cannot be the only path. The exported payload is populated, not a name-only stub.

**Acceptance**
- On a **default** install (Commands off, matching D-012), menu bar or Settings recovery entry runs Doctor and reports hotkey registration state, corrupt config files, and latency p95.
- Export diagnostics from that recovery entry writes `~/Library/Logs/Luma/diagnostics.json` with populated `platform`, `modules`, `permissions`, `recentErrors`, and `corruptConfigFiles` fields — not `nil`/empty placeholders.
- Optional mirrored `cmd doctor` / `cmd export-diagnostics` entries may exist when Commands is enabled, but P0 acceptance does not depend on Commands being enabled.
- `crash-log.txt`'s actual path (`~/Library/Application Support/Luma/crash-log.txt`) is what the exported diagnostics/doctor output references — no more path mismatch between code and what the user is told to look at.
- `CrashLogBuffer.persist()` write failures are either surfaced (not purely silent `try?`) or explicitly documented as an accepted, isolated failure mode with a named reason (C-DIAG-003 is a SHOULD, not MUST — a documented decision is an acceptable outcome).
- `ConfigCorruptionRegistry`'s in-memory-only nature and the split quarantine paths (`JSONConfigPersistence` vs `ClipboardHistoryStore` vs `JSONFileStore`) are at minimum surfaced as a known-incomplete corruption view in doctor output — full unification across all three paths is **P3** scope, but doctor must not claim complete coverage it does not have.

**Risk** — The remaining implementation choice is placement within the accepted surfaces (menu bar, Settings, or both). Do not turn this into a broader Commands redesign; user scripts stay parked/default-off.

**Dependencies** — P0.1-P0.3 for signed-app verification; this task itself **blocks all effective debugging of every other P0/P1/P2 task**, so it should be sequenced early within P0, not last.

**Non-goals** — Do not build a general-purpose scripting/commands platform; do not unify all three quarantine paths into one persistence format (that is **P3** territory); do not add telemetry or remote reporting — diagnostics stay local-only per `docs/ENGINEERING.md` privacy section.

## 6. P1 — 降低核心复杂度

### P1.1 LauncherRootController Boundary

**Problem / 现象** — `LauncherRootController` owns the `LauncherViewModel` reference, `LauncherContentCoordinator`, `LauncherSnapshotApplyPipeline`, `LauncherDetailPresenter`/`LauncherDetailLifecycleController`, `LauncherTaskRegistry`, and three home providers (`OpenAppsHomeProvider`, `ClipboardPasteboardCache`, `LauncherHomeCoordinator`) simultaneously. It is also the file with the most direct citations across `ARCHITECTURE_MAP.md`'s show/hide, query, and detail traces — meaning nearly every cross-cutting concern eventually routes through it.

**Evidence / 证据来源** — `ARCHITECTURE_MAP.md` lines 199-232 (ownership graph and Mermaid diagram); `PRODUCT_FLOWS.md` Cross-Cutting State Owners table (multiple rows name `LauncherRootController.swift` alongside other owners); `CONTRACTS.md` C-UI-001..006, C-ASYNC-001..003; direct grep in this phase: `LauncherRootController.swift` has 4 `Task { @MainActor` sites, consistent with it being a coordination hub rather than a single-purpose controller.

**Owner Area** — `Sources/LumaApp/Launcher/LauncherRootController.swift` and its direct collaborators listed above.

**Desired Outcome** — `LauncherRootController`'s responsibility list is written down and shortened to what it actually must own (wiring/dispatch), with query/selection/detail/visibility/session-state ownership made explicit elsewhere (see P1.2/P1.3) rather than folded into this one type by default.

**Acceptance**
- A written responsibility list for `LauncherRootController` exists (in `docs/ENGINEERING.md` or an equivalent) and is shorter than today's implicit list (query handling + snapshot pipeline + detail presenter/lifecycle + task registry + 3 home providers, all in one type).
- For each of query, selection, detail, visibility, and session state, the owning type is named and matches what P1.2/P1.3 establish — no state is described as "owned by `LauncherRootController`" once P1.2/P1.3 land, except wiring/orchestration.
- The divergence between `LauncherFlowHarness`'s test wiring and `AppCoordinator`'s production wiring (empty `CommandRegistry`, missing `configureGlobalSearchModuleIDs`, per C-TEST-004) is either closed or explicitly labeled in test code as "logic-only, not production-representative."

**Risk** — `LauncherRootController` is the single most heavily cross-referenced file in `ARCHITECTURE_MAP.md`; narrowing its boundary without breaking show/hide, query dispatch, or detail lifecycle requires careful, incremental extraction, not a single large diff. There is a real risk of re-triggering the P0.1 crash class if AppKit-adjacent code in this file is moved carelessly across actor boundaries.

**Dependencies** — P0 fully exited (this task touches the exact code paths implicated in the current crashes and latency regression, so it must be provably safe to change only after the runtime is stable and diagnosable).

**Non-goals** — Do not split `LauncherRootController` into a large number of new types speculatively; do not change its public API surface used by `LauncherRootView`/`AppCoordinator` unless required; do not attempt this task without diagnostics (P0.8) already reachable to catch regressions.

---

### P1.2 Launcher Session State Owner

**Problem / 现象** — Query text, selection index, content mode, and visibility each currently have a *documented* single owner (`LumaSearchBar`→`LauncherRootController`→`LauncherViewModel` for query; `LauncherContentCoordinator` for content mode; `LauncherListView`↔`LauncherContentCoordinator.selectedIndex` for selection; `LauncherWindowController`+`LauncherPanelVisibilitySession` for visibility) — but `CONTRACTS.md` records deviations against nearly every one of these owners (C-UI-001 menu-bar-Show bypass; C-UI-003 doc/code location mismatch for `LauncherContentMode`; C-UI-004 stale-selection-to-index-0 fallback risk). The ownership model exists on paper but has confirmed cracks.

**Evidence / 证据来源** — `CONTRACTS.md` C-UI-001 through C-UI-006 and their Current Known Deviations; `PRODUCT_FLOWS.md` Cross-Cutting State Owners table and Flow 8 (selection); `USABILITY_TRIAGE.md` S-013/S-014 (Inferred Risk, detail/hide state).

**Owner Area** — `Sources/LumaApp/Launcher/LauncherContentCoordinator.swift`, `Sources/LumaCore/Home/LauncherKeyRouter.swift` (holds `LauncherContentMode` type), `Sources/LumaApp/Launcher/LauncherListView.swift`, `Sources/LumaCore/Home/LauncherPanelVisibilitySession.swift`.

**Desired Outcome** — Each of query/selection/mode/visibility/empty-loading-error-result state has one enforced owner with no direct-mutation bypass, closing the specific deviations C-UI-001/003/004/006 record today.

**Acceptance**
- The `LauncherContentMode` type-location doc/code mismatch (C-UI-003) is resolved: either the type moves to `LauncherContentCoordinator` or the docs are corrected to name `LauncherKeyRouter.swift` as the type's home — pick one and make them match.
- The menu bar Show bypass of `showFromCarbonHotkey()`'s guard (C-UI-001/006) is either removed (routed through the same guarded API) or proven safe with a test that exercises rapid Show+hotkey sequencing and asserts no inconsistent visibility state.
- The stale-selection-fallback-to-index-0 risk (C-UI-004) has a test that asserts Return after a snapshot removes the previously-selected item does not silently execute a different, unintended row.
- Return, Esc, hide, and re-summon in sequence do not produce a stale selection or a wrong content mode, verified by an automated test exercising this specific sequence.

**Risk** — Moving `LauncherContentMode`'s type definition is a small change but touches every file that imports it; the menu-bar-Show bypass fix could regress the "fallback path when hotkey fails" behavior that P0.2 depends on — coordinate with P0.2's acceptance criteria so the fallback still works after this fix.

**Dependencies** — P0 fully exited; loosely coordinates with P1.1 (same files) and P0.2 (menu bar Show).

**Non-goals** — Do not introduce a new state-management framework (Redux-style, Combine-heavy, etc.); do not change the SearchBar→RootController→ViewModel query path shape (C-UI-002 has no recorded deviation — leave it alone).

---

### P1.3 Detail Lifecycle

**Problem / 现象** — Detail presentation is split four ways: `ModuleDetailRegistry` (pooled instances + content-generation guard), `LauncherDetailPresenter` (presentation-generation + payload staging), `LauncherDetailLifecycleController` (close-crossfade sequencing), and `LauncherContentCoordinator` (UI-hierarchy attach/hide). `contentCoordinator.closeDetail` is called only from the lifecycle controller's `tearDownAfterGuideCrossfade`, while a separate `tearDownDetailIfNeeded` (called from `showHome`) does a harder `removeFromSuperview` teardown — two different teardown strengths coexist for what looks like one concept.

**Evidence / 证据来源** — `PRODUCT_FLOWS.md` Cross-Cutting State Owners (Detail lifecycle row, "four-way split") and lines 753-772 (component responsibilities table, the two teardown functions); `CONTRACTS.md` C-DETAIL-001 through C-DETAIL-005; `USABILITY_TRIAGE.md` S-012/S-013 (Needs user confirmation).

**Owner Area** — `Sources/LumaApp/Composition/ModuleDetailRegistry.swift`, `Sources/LumaApp/Launcher/Session/LauncherDetailPresenter.swift`, `LauncherDetailLifecycleController.swift`, `Sources/LumaApp/Launcher/LauncherContentCoordinator.swift`.

**Desired Outcome** — The four-way split is documented as an intentional layering (registry=pooling, presenter=generation/payload, lifecycle controller=crossfade sequencing, coordinator=hierarchy) with the two teardown paths (`closeDetail` vs `tearDownDetailIfNeeded`) either unified or clearly distinguished by name and contract, so a future contributor does not have to reverse-engineer which one to call.

**Acceptance**
- A single written description exists (in `docs/ENGINEERING.md`) naming what each of the four detail-lifecycle types owns and is not allowed to do (per C-DETAIL-001: detail views must not drive module lifecycle in reverse).
- `closeDetail` (pooled hide) and `tearDownDetailIfNeeded` (hard `removeFromSuperview`) are renamed or documented so their different strength is obvious from the call site, not just from reading their bodies.
- Opening a module with no registered detail does not advertise an open-detail affordance (C-DETAIL-005); this is verified by a review/test cross-check of `ModuleDetailRegistry.makeDefault()` registrations against each module's `bareBehavior`.
- Detail warmup/teardown does not trigger heavyweight module lifecycle work beyond what the module's own `warmup`/`teardown` contract already allows (C-DETAIL-001) — verified for at least the MVP modules (Clipboard, Notes, Todo, Translate, Snippets, Quicklinks).

**Risk** — This is one of the more test-covered flows already (`DetailHierarchy`, `LauncherSearchDetailMode`, `LauncherDetailExitPlanner` per C-DETAIL-002/003/004 having "no deviation" status) — the risk here is spending effort on the well-covered parts (open/reuse/exit) instead of the actually under-specified part (the two teardown functions and the four-way split's documentation gap).

**Dependencies** — P0 fully exited; benefits from P1.1's narrower `LauncherRootController` boundary but does not strictly require it to complete first.

**Non-goals** — Do not merge the four detail-lifecycle types into one; do not change `LauncherDetailExitPlanner`'s outcome logic (`restoreSuspendedQuery`/`returnToHome`/`reenableSearchOnly`) — it has no recorded deviation and works as documented.

---

### P1.4 Task / MainActor Boundary Cleanup

**Problem / 现象** — A direct grep in this phase found **89 `Task { @MainActor` occurrences across 30 files** in `Sources/LumaApp`, with `NotesDetailView.swift` alone containing 23. A direct run of `scripts/scan_appkit_executor_risk.sh` found **89 warn-only findings**: `@objc` target/action methods inside `@MainActor` AppKit subclasses across 12 detail-view files that "should be nonisolated" per `docs/swift6-appkit-boundaries.md`. These are warn-only today (the script's blocking checks pass), but they are exactly the risk class implicated in the three `.ips` crashes from Phase 0.

**Evidence / 证据来源** — Direct tool execution in this phase (see Section 2); `CONTRACTS.md` C-APPKIT-001/002/003 and Current Known Deviation #15-16; `docs/swift6-appkit-boundaries.md` (the ADR these findings violate); `PRODUCT_FLOWS.md` note under Flow 3 ("`Task { @MainActor in ... }` patterns are visibly applied throughout the files read directly in this phase").

**Owner Area** — The 12 files flagged by the scanner (`ClipboardDetailView`, `SnippetsDetailView`, `WordbookDetailView`, `TranslateDetailView`, `TodoDetailView`, `MediaDetailView`, `SecretsDetailView`, `ProjectsDetailView`, `CurrentProjectDetailView`, `QuicklinksDetailView`, plus the two file-level list-scroll helpers); the 30 files with `Task { @MainActor }` sites, prioritized by MVP relevance (Clipboard and Notes first, then Snippets/Quicklinks/Translate/Todo depending on the Core P1 and Todo Open Decisions; Wordbook/Media/Secrets/Projects/CurrentProject are parked-module files and lower priority per Section 4's ordering rules). Notes is explicitly included because `NotesMindMapView.isFlipped` appears in a same-day `.ips` crash frame even though it is not the same warn-only scanner category.

**Desired Outcome** — The MVP-relevant subset of the 89 warn-only `@objc`-in-`@MainActor` findings is resolved (methods marked `nonisolated` per the ADR), and `Task { @MainActor }` usage in MVP-path files is confirmed to be boundary-bridging (per C-APPKIT-003) rather than ad-hoc state-fixing.

**Acceptance**
- `scripts/scan_appkit_executor_risk.sh` warn-only findings for the MVP-path files (Clipboard detail view, plus Snippets/Translate/Quicklinks and Todo if those Core P1/conditional surfaces are retained) drop to zero; Notes' AppKit crash-adjacent surface (`NotesDetailView` / `NotesMindMapView`) is separately reviewed and smoke-tested even if it is not represented by the same warn-only scanner rule. Findings in parked-module files (Wordbook, Media, Secrets, Projects, CurrentProject) are explicitly logged as a registered, lower-priority backlog rather than fixed under P1 time pressure.
- A manual review of `Task { @MainActor }` sites in `LauncherRootController.swift`, `LauncherWindowController.swift`, `AppCoordinator.swift`, and `NotesDetailView.swift` (the highest-traffic and highest-count files) confirms each site is a legitimate nonisolated-callback-to-MainActor bridge, not a state-ordering workaround (C-APPKIT-003) — any found to be the latter are flagged with a follow-up, not silently left.
- No new AppKit executor-boundary `.ips` crash occurs during the P0/P1 smoke passes after this cleanup.
- `docs/swift6-appkit-boundaries.md` remains the single source of truth; this task does not introduce a second, competing boundary-rules document.

**Risk** — Marking 89 methods `nonisolated` individually is mechanical but not zero-risk — some of these `@objc` methods may genuinely need MainActor state and require a `Task { @MainActor }` bridge inside the now-nonisolated method rather than a blind annotation change; each site needs a real read, not a scripted find-replace.

**Dependencies** — P0.1 (this directly touches the crash-risk class P0.1 must have already stabilized); should follow P1.1 (fewer moving parts in `LauncherRootController` while doing MainActor-boundary surgery reduces conflict risk) but can run in parallel with P1.3.

**Non-goals** — Do not fix every one of the 89 findings in a single pass; do not touch parked-module files (Wordbook/Media/Secrets/Projects) as part of MVP-path work; do not change `docs/swift6-appkit-boundaries.md`'s rules themselves — this task brings code into compliance with existing rules, it does not rewrite the rules.

---

### P1.5 Cache Refresh vs UI Repaint Separation

**Problem / 现象** — Background Open Apps refresh, per-module warm caches, and `QuerySnapshotCache` stale-while-revalidate all currently interact with UI repaint through several different paths (`LauncherHomeCoordinator`, `LauncherSnapshotApplyCoalescer`, per-module `handle()` cold-state rows). The confirmed hotkey p95 ≈ 8.3 s regression is the strongest evidence that *something* in the show/first-frame chain is not properly separated from cache refresh work, though root cause is explicitly undiagnosed per Phase 0-6.

**Evidence / 证据来源** — `CONTRACTS.md` C-ASYNC-003 ("Background refresh does not repaint hidden panel or block hotkey→visible") and its Current Known Deviation ("hotkey p95 ≈ 8.3 s ... a standing violation ... whose root cause is deliberately not diagnosed in Phase 0-3"); `docs/ENGINEERING.md` Performance Contract hot-path rules (Open Apps refresh bound to visibility, no rebuild on first visible frame); `CONTRACTS.md` C-CACHE-001/002.

**Owner Area** — `Sources/LumaApp/Launcher/HomeProviders/OpenAppsHomeProvider.swift`, `LauncherHomeCoordinator.swift`, `ClipboardPasteboardCache.swift`; `Sources/LumaCore/Query/QuerySnapshotCache.swift`.

**Desired Outcome** — Cache refresh and UI repaint are demonstrably decoupled: refresh failures or slowness never block the hot path, and the latency report can distinguish "waiting for input" from "waiting for a background refresh."

**Acceptance**
- Each cache (`QuerySnapshotCache`, Open Apps cache, per-module warm caches named in `MODULE_MATRIX.md`) has a written owner, scope, and TTL/invalidation trigger (C-CACHE-001) — most already do per docs; this task confirms and documents any gaps found.
- A background refresh failure (simulated: force a cache miss or slow refresh) does not put the UI into an unexplained state — it shows a warming/degraded row per C-FAIL-002, never a freeze.
- Query hot path (`handle()`, `QueryDispatcher.dispatch`) does not await any refresh task — confirmed by the existing `ModuleHandleContractTests`/`browserTabsHandleUsesCacheOnlyPath`-style tests plus, if a gap is found, a new equivalent test for Open Apps' home refresh path.
- The new `latency-report.json` captured after P0.2's hotkey fix (see P0.2) is used here to confirm the fix's mechanism is cache/repaint separation, not a workaround that happens to reduce the number without addressing C-ASYNC-003's root contract.

**Risk** — This task overlaps directly with P0.2's hotkey-latency fix; if P0.2 already root-causes and fixes the 8.3 s regression, P1.5's scope shrinks to "confirm and document" rather than "diagnose and fix" — check P0.2's outcome before scoping this task's implementation work.

**Dependencies** — P0.2 (hotkey latency fix) must land first; depends on P0.1/P0.3 for a stable, measurable runtime.

**Non-goals** — Do not build a generic caching framework or abstraction layer; do not change per-module cache implementations beyond what's needed to prove hot-path independence; do not attempt to fix every TTL/cache inconsistency found — only ones that violate C-CACHE-001/002 for MVP-path modules.

## 7. P2 — 模块治理

> **Canonical execution order (Phase 14, 2026-07-07):** `P2_ROADMAP.md` is the source of truth for **what to do next**. Subsections below use the **same P2.1–P2.5 numbering** as Phase 14. The original Phase 7 investigation used a different order (lifecycle first, defaults as P2.3); that order is **superseded** — do not start at old "lifecycle = P2.1". Scope audit: `P2_SCOPE_AUDIT.md`. Decisions: `P2_DECISION_MATRIX.md`.

### P2.1 Documentation / Manifest Hygiene — ✅ Complete (`8539007c`)

**Maps to:** Phase 14 P2.1 · formerly Phase 7 **P2.3** (Default Enabled Module Slimming)

**Problem / 现象** — `docs/PERMISSIONS.md`'s Default column marks Menu Items, Wordbook, WindowLayouts, Media, Projects, KillProcess, Secrets, and Workbench as "on," directly contradicting `defaultEnabled: false` in manifests and D-012's current default-on/default-off list — confirmed by direct read of `docs/PERMISSIONS.md` in this phase. `WindowsModule`'s manifest declares `defaultEnabled: true` while the module is not registered in `ModuleRegistry.allBundles` at all (only in `BuiltInModules.makeDeferred()`). Parked/deferred registration status is not consistently documented.

**Evidence / 证据来源** — Direct read of `docs/PERMISSIONS.md` in this phase (Default column); `CONTRACTS.md` C-DEFAULT-004 and C-MODULE-006 and their Current Known Deviations (#1, #2 in the consolidated list); `MODULE_MATRIX.md` Code/Docs Mismatches section; `docs/MODULES.md` current D-012 default-on/default-off lists (which already agree with code); `LAUNCHER_STATE_AUDIT.md` (session state test-only decision).

**Owner Area** — `docs/PERMISSIONS.md` (doc fix), `docs/MODULES.md` (registration status table), `Sources/LumaModules/Windows/` manifest (the misleading `defaultEnabled: true` flag on a deferred module).

**Desired Outcome** — The default-on/off state for every module is identical across `docs/PERMISSIONS.md`, `docs/MODULES.md`, manifests, and `ModuleWarmupDefaults` — closing C-DEFAULT-004 for good, and correcting `WindowsModule`'s misleading manifest flag without registering it. Deferred/parked modules are explicitly labeled. `LauncherSessionState` remains **test-only** (no new production wiring in P2.1).

**Acceptance**
- `docs/PERMISSIONS.md`'s Default column is corrected to match current code and D-012 for every module, while explicitly distinguishing current default-on state from MVP P0 scope: Apps/Clipboard/Notes are P0 core modules; Snippets/Quicklinks/Translate are Core P1 candidates; Todo follows the `MVP_SCOPE.md` Open Decision until the user decides whether it stays default-on or moves out of the active MVP path. Commands/Media/BrowserTabs/MenuItems/WindowLayouts/Wordbook/Secrets/KillProcess/Projects remain off. This is a **documentation correction**, not a manifest/code change, per the "do not change any module's default on/off switch" boundary already set in `MVP_SCOPE.md`.
- `docs/PERMISSIONS.md`'s module naming ("Menu Items" vs code/`docs/MODULES.md`'s "Menu Bar Search") is corrected to match.
- `WindowsModule`'s manifest `defaultEnabled` flag is corrected to `false` (or an equivalent "deferred" marker) to stop misrepresenting a module that is not even registered — this is a P2 hygiene correction, not a Phase 9 P0 starter and not a re-entry of the module into the default path; `WindowsModule` remains unregistered in `ModuleRegistry.allBundles`.
- After this task, `MVP_SCOPE.md`'s "Current mismatches (target ≠ current fact)" list items for `docs/PERMISSIONS.md` stale defaults and the Windows manifest flag are both resolved.

**Risk** — Very low technical risk (this is primarily a documentation fix plus one manifest flag correction on an already-deferred module); the risk is scope creep into "let's also reconsider which modules should be default-on" — that is explicitly not this task (see Decision Summary #4/#11).

**Dependencies** — None blocking; **must complete before P2.2–P2.5 code slices** per `P2_ROADMAP.md`.

**Non-goals** — Do not change any module's actual `defaultEnabled` value in a way that changes runtime behavior (except the Windows manifest flag, which does not change runtime behavior since the module is unregistered); do not register `WindowsModule`; do not wire `LauncherSessionState` production events; do not resolve the Todo/Translate/Snippets/Quicklinks tier Open Decisions inside this governance task.

---

### P2.2 Module Diagnostic Consistency — ✅ Complete (`8539007c`)

**Maps to:** Phase 14 P2.2 · same topic as Phase 7 **P2.2**

**Problem / 现象** — Diagnostic/failure behavior is inconsistent: non-`.queryable` targeted modules return silent empty (no diagnostic) while disabled modules return a diagnostic row; some modules emit bespoke cold-state strings while the dispatcher's generic `module.warming` row exists in parallel; "empty acceptable" vs "diagnostic required" is decided ad hoc per module in `MODULE_MATRIX.md`, not enforced by any shared mechanism.

**Evidence / 证据来源** — `CONTRACTS.md` C-FAIL-005 and its Current Known Deviations; `PRODUCT_FLOWS.md` Flow 7 (targeted module search) and Flow 14 (module cold start); `MODULE_MATRIX.md` Classification Legend / Diagnostic Requirements section.

**Owner Area** — `Sources/LumaCore/Results/ModuleDiagnosticResults.swift`, `PermissionResultBuilder.swift`, `Sources/LumaCore/Query/QueryDispatcher.swift` (the `module.warming` / timeout-diagnostic synthesis path).

**Desired Outcome** — The MVP Core P0 modules share one documented failure taxonomy (permission-required, degraded/warming, onboarding, timeout, empty-acceptable) with no module inventing ad-hoc semantics that diverge from it; Core P1 / conditional candidates must satisfy the same taxonomy before being treated as active MVP surfaces.

**Acceptance**
- A single documented failure-behavior table exists (in `docs/ENGINEERING.md` or `docs/MODULES.md`) that each MVP Core P0 module row references, and that Core P1 / conditional candidates reference before activation — this closes the specific gap C-FAIL-005 names ("no single documented failure-behavior table").
- For the MVP module set: disabled → diagnostic row; permission blocked → diagnostic row (C-FAIL-001); cold cache → warming/degraded row distinct from empty (C-FAIL-002); unconfigured (Notes) → onboarding row (C-FAIL-003); timeout → diagnostic, never silent (C-HOT-005). Each is verified, not assumed.
- The "non-`.queryable` targeted module returns silent empty" behavior (C-HOT-004 Current Known Deviation) is resolved for MVP modules specifically — either give it a diagnostic or confirm the module state is unreachable in the MVP configuration.

**Risk** — Modules outside the MVP set (Wordbook, Media, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows) share the same `ModuleDiagnosticResults`/`PermissionResultBuilder` infrastructure; changes here could affect them even though they are out of scope — test the parked modules' existing behavior is unchanged, without investing in improving it.

**Dependencies** — **P2.1** complete.

**Non-goals** — Do not build a new diagnostic-kind enum or taxonomy from scratch — extend/document the existing `ModuleDiagnosticResults`/`ModuleDiagnostic` kinds; do not fix diagnostic behavior for parked modules.

---

### P2.3 Module Lifecycle Contract Tests — ✅ Complete (`8539007c`)

**Maps to:** Phase 14 P2.3 · formerly Phase 7 **P2.1** (Unified Module Lifecycle Contract)

**Problem / 现象** — `warmup`/`handle`/`perform`/`teardown` responsibilities are documented as distinct (C-MODULE-002/003) but have per-module exceptions: `KillProcessModule` has no explicit `teardown` despite scheduling refresh tasks; `WordbookModule.perform` throws `unsupportedAction` because review runs in detail instead; `WindowsModule.handle` calls `CGWindowListCopyWindowInfo` directly (deferred/unregistered, must stay that way per Decision Summary #6).

**Evidence / 证据来源** — `CONTRACTS.md` C-MODULE-001 through C-MODULE-006 and their Current Known Deviations; `MODULE_MATRIX.md` per-module lifecycle notes (Windows, KillProcess, Wordbook sections).

**Owner Area** — `Sources/LumaCore/Modules/LumaModule.swift` (protocol), `Sources/LumaModules/*/` per-module actors, especially the MVP Core P0 modules (Apps, Clipboard, Notes) and the Core P1 / conditional candidates (Snippets, Quicklinks, Translate, Todo).

**Desired Outcome** — Every MVP Core P0 module conforms to the same warmup/handle/perform/teardown contract shape with no undocumented exceptions; Core P1 / conditional candidates must meet the same bar before they are allowed into the active MVP path. Deviations in parked modules are logged but not fixed (they don't block MVP).

**Acceptance**
- Each MVP Core P0 module (Apps, Clipboard, Notes) has explicit, verifiable warmup/handle/perform/teardown behavior matching C-MODULE-002/003 — verified via existing or new `ModuleHandleContractTests`/`ModuleColdCacheTests`-style coverage.
- `handle()` memory-only compliance for the MVP Core P0 set has at least a proxy test each (C-HOT-001), closing the gap noted in Current Known Deviation #9 ("only per-module proxy tests ... no generic enforcement") for the modules that actually block P0 specifically.
- Snippets, Quicklinks, Translate, and Todo are treated as Core P1 / conditional candidates: they must satisfy the same lifecycle and memory-only checks before being enabled as part of the active MVP path, but they do not expand the P0 recovery scope by default.
- Every module Task in the MVP set has a named owner and confirmed cancellation point (C-MODULE-004) — this closes the "not re-verified" status on tasks like the menu-tree context Task only if that task is in the MVP path; if it is not, it stays out of scope.
- Deferred/parked modules' known deviations (Windows CGWindow call, KillProcess missing teardown) remain recorded facts, not fixed — fixing them would be pulling deferred work into a governance pass in violation of Decision Summary #4/#6.

**Risk** — "Unify the contract" can silently balloon into touching every module; the acceptance criteria above scope this strictly to the P0 modules first, with Core P1 / conditional candidates gated before activation, to prevent that.

**Dependencies** — P0 fully exited; P1 core-complexity work substantially done; **P2.1** and **P2.2** complete.

**Non-goals** — Do not fix `WindowsModule`'s `CGWindowListCopyWindowInfo` violation (it is deferred and must stay deferred); do not add `teardown` to `KillProcessModule` (it is default-off/parked); do not touch `WordbookModule.perform`'s `unsupportedAction` design (parked module); do not rewrite `ModuleHost`.

---

### P2.4 Non-MVP / Core P1 AppKit Cleanup — ✅ Complete (`8539007c`)

**Maps to:** Phase 14 P2.4 · extends P1.4 (Clipboard MVP only) to Core P1 detail views

**Problem / 现象** — `scripts/scan_appkit_executor_risk.sh` reports ~78 warn-only `@objc` target/action methods inside `@MainActor` AppKit subclasses across detail views (Snippets, Wordbook, Translate, Todo, Media, Secrets, etc.). Clipboard MVP paths were fixed in P1.4; parked-module bulk cleanup is out of scope.

**Owner Area** — Core P1 detail views: `SnippetsDetailView`, `QuicklinksDetailView`, `TranslateDetailView`, `TodoDetailView` (one file per PR).

**Desired Outcome** — MVP and Core P1 detail surfaces comply with `docs/swift6-appkit-boundaries.md` without a single large sweep.

**Acceptance** — Touched file: zero scanner warns for that file; `swift build`; targeted module smoke if available.

**Dependencies** — P2.1–P2.3 substantially complete (avoid mixing with lifecycle/diagnostic refactors in same PR).

**Non-goals** — Parked modules (Wordbook, Media, Secrets, Projects); scanner rule changes; `docs/swift6-appkit-boundaries.md` rewrite.

*Full slice spec: `P2_ROADMAP.md` § P2.4.*

---

### P2.5 QA Harness / Smoke Runner — ✅ Complete (`8539007c`)

**Maps to:** Phase 14 P2.5 · partial overlap with P3.2/P3.4

**Problem / 现象** — `LUMA_QA_*` smokes write JSON but do not terminate the app; `LauncherFlowHarness` diverges from production wiring (C-TEST-004).

**Owner Area** — `scripts/run_p0_smokes.sh` (new), optional auto-exit in `*ProductionSmoke.swift`, `LauncherFlowHarness.swift`.

**Desired Outcome** — Single terminable P0 smoke command; harness documents or achieves production router parity.

**Acceptance** — `./scripts/run_p0_smokes.sh` exits 0/1 with JSON artifacts; `docs/QA.md` references script.

**Dependencies** — P2.1–P2.4 slices stable; P0 gate green.

**Non-goals** — Full AppCoordinator E2E framework; CI macOS runner requirement in first slice.

*Full slice spec: `P2_ROADMAP.md` § P2.5.*

---

### Legacy backlog — Module Detail View Layering (Phase 7 P2.4)

> **Superseded for P2 execution order.** Not part of Phase 14 P2.1–P2.5. Defer to **P3** or product backlog unless C-DETAIL-005 becomes a P0 regression.

**Problem / 现象** — Whether any no-detail module's open-detail path is reachable via normal user flow is unconfirmed (C-DETAIL-005 deviation); `Apps`, `Commands`, `MenuItems`, `KillProcess`, `BrowserTabs`, `WindowLayouts`, and `Windows` have no registered detail view per `MODULE_MATRIX.md`'s Summary Matrix.

**Dependencies** — P1.3 (Detail Lifecycle) documented.

**Non-goals** — Do not build new detail views for currently no-detail modules under P2.

---
## 8. P3 — 文档和测试对齐

### Phase 17 — P3.1 docs governance (2026-07-07)

| Deliverable | Status |
|-------------|--------|
| Diagnostics ownership + paths (`docs/ENGINEERING.md`, `docs/PERMISSIONS.md`) | ✅ |
| `LauncherContentMode` / `LauncherSessionState` docs | ✅ |
| Recovery entry + smoke gate docs (`docs/QA.md`, `docs/MODULES.md`) | ✅ |
| `CONTRACTS.md` resolved deviations sweep | ✅ |
| `P3_DOCS_GOVERNANCE_REPORT.md` | ✅ |

**P3.1 verdict:** **Go** — see `P3_DOCS_GOVERNANCE_REPORT.md`. **Next:** P3.2 test organization.

### P3.1 Remove / Rewrite Stale Docs — ✅ Complete (Phase 17)

**Problem / 现象** — Beyond the `docs/PERMISSIONS.md` default-column staleness (P2.1), other doc/code mismatches exist: `LauncherContentMode`'s documented location (`docs/ENGINEERING.md` says "in `LauncherContentCoordinator`") vs actual location (`LauncherKeyRouter.swift`); diagnostics ownership documented as `LumaInfrastructure` while `DiagnosticsPayload`/`DiagnosticsExport`/`CrashLogRecording` actually live in `LumaCore/Util`; `crash-log.txt`'s actual path undocumented anywhere.

**Evidence / 证据来源** — `CONTRACTS.md` Current Known Deviations #5, #17; `ARCHITECTURE_MAP.md` layer cross-check section; `MODULE_MATRIX.md` Code/Docs Mismatches section.

**Owner Area** — `docs/ENGINEERING.md`, `docs/PERMISSIONS.md`, `docs/MODULES.md`.

**Desired Outcome** — Every doc/code location and default-state mismatch recorded in `CONTRACTS.md`'s Current Known Deviations list is resolved by correcting the doc to match code (per Decision Summary #8: docs follow code, never the reverse), using `CONTRACTS.md`/`MVP_SCOPE.md`/this plan as the arbiter.

**Acceptance**
- `docs/ENGINEERING.md`'s `LauncherContentMode` location description matches its actual code location (or the code is moved to match docs — pick whichever is decided in P1.2, then align the doc to that outcome).
- `docs/ENGINEERING.md`'s diagnostics/logging layer-ownership description matches where `DiagnosticsPayload`/`DiagnosticsExport`/`CrashLogBuffer` actually live.
- `crash-log.txt`'s path (`~/Library/Application Support/Luma/crash-log.txt`) is documented in `docs/ENGINEERING.md` and/or `docs/PERMISSIONS.md`, resolving C-DIAG-004.
- No document describes a parked module (Media, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench) as part of the MVP main path.
- Every remaining item in `CONTRACTS.md`'s "Current Known Deviations" section that is doc-only (not a code-behavior deviation) is either resolved or has an explicit note in `docs/DECISIONS.md` re-deciding it.

**Risk** — Low technical risk; the main risk is doing this before P1.2's `LauncherContentMode` decision lands, which would mean redoing this doc fix twice — sequence after P1.2.

**Dependencies** — P1.2 (content-mode decision: docs corrected to match code, P3.1), P0.8 (diagnostics paths), P2.1 (defaults).

**Non-goals** — Do not rewrite `docs/ENGINEERING.md`/`docs/MODULES.md` wholesale; do not delete `docs/DECISIONS.md` history; do not add new documentation files beyond correcting existing ones and `P3_DOCS_GOVERNANCE_REPORT.md`.

---

### Phase 18 — P3.2 test organization (2026-07-07)

| Deliverable | Status |
|-------------|--------|
| `docs/QA.md` § MVP Flow Test Map | ✅ |
| `P3_TEST_ORGANIZATION_REPORT.md` | ✅ |
| Harness partial parity + backlog explicit | ✅ |

**P3.2 verdict:** **Go** — see `P3_TEST_ORGANIZATION_REPORT.md`. **Next:** P3.3 performance budgets or P3.4 release hardening.

**P3 backlog (not stolen into P3.2):** Full `LauncherFlowHarness` ↔ `AppCoordinator` parity; full UI E2E framework.

### P3.2 Reorder Tests Around Main Flows — ✅ Complete (Phase 18)

**Problem / 现象** — `LauncherFlowHarness` builds its own `ModuleHost`/`QueryDispatcher`/`LauncherViewModel` stack that diverges from production (empty `CommandRegistry`, missing `configureGlobalSearchModuleIDs`, different Apps-warmup timing), and `launcherFlowHarnessReplaysQuery` has already flipped between failing and passing across phases with no attributed cause — direct evidence that harness-green does not equal production-correct (C-TEST-004).

**Evidence / 证据来源** — `CONTRACTS.md` C-TEST-001/004 and Current Known Deviation #13; `PRODUCT_FLOWS.md` Flow 6 harness-divergence description; `CURRENT_STATE.md`/`USABILITY_TRIAGE.md` test-result flip across phases (S-026).

**Owner Area** — `Tests/LumaAppTests/Flow/LauncherFlowHarness.swift`, `docs/QA.md` (test organization and release checklist).

**Desired Outcome** — Tests are discoverable by MVP main flow (startup, hotkey, input, search, action, diagnostics), not only by target/package, and `LauncherFlowHarness`'s divergence from production is either closed or explicitly labeled.

**Acceptance** — **Met (Phase 18):** `docs/QA.md` § MVP Flow Test Map; `P3_TEST_ORGANIZATION_REPORT.md`; harness gaps labeled; signed-app smoke referenced as mandatory release gate. Full harness parity **deferred** to P3 backlog.

**Risk** — Reconciling `LauncherFlowHarness` with production wiring could be substantial work if the divergence is deep (it touches `CommandRegistry` and global-search tier configuration); if it proves too large for P3, the fallback (explicit "logic-only" labeling) is an acceptable, honest alternative — do not let this block P3 exit. *(Phase 18 took the labeling path.)*

**Dependencies** — P0/P1 substantially complete (test reorganization around flows that are themselves unstable would need redoing).

**Non-goals** — Do not rewrite the entire test suite; do not delete existing passing tests; do not require 100% flow coverage — C-TEST-001 requires automated test *or* explicit manual QA per P0 flow, not automated-only.

---

---

### Phase 19 — P3.3 performance budgets (2026-07-07)

| Deliverable | Status |
|-------------|--------|
| `docs/ENGINEERING.md` release vs aspirational performance tables | ✅ |
| `docs/QA.md` § Performance Gate | ✅ |
| `scripts/qa/export_latency_report.sh` `LUMA_RELEASE_GATE=1` | ✅ |
| `P3_PERFORMANCE_BUDGETS_REPORT.md` | ✅ |

**P3.3 verdict:** **Go** — see `P3_PERFORMANCE_BUDGETS_REPORT.md`.

### P3.3 Keep Only Critical Performance Budgets — ✅ Complete (Phase 19)

**Problem / 现象** — `docs/ENGINEERING.md`'s Performance Contract table lists five budgets (hotkey→interactive, hotkey→home, keystroke→paint, module `handle`, panel-hide-after-action); the confirmed regression (hotkey p95 ≈ 8.3 s) is on exactly one of these five, while the others (keystroke, module handle, panel hide) are not confirmed to be tracked with the same rigor in `latency-report.json`, which currently records only `hotkeyP95Milliseconds`, `keystrokeP95Milliseconds`, and `combinedP95Milliseconds`.

**Evidence / 证据来源** — `docs/ENGINEERING.md` Performance Contract budgets table; `CURRENT_STATE.md` `latency-report.json` key fields (only 3 of 5 budget categories represented); `CONTRACTS.md` C-HOT-002/C-ASYNC-003.

**Owner Area** — `Sources/LumaApp/Infrastructure/LatencyHUD.swift` (`LatencyTelemetry.exportReport()`), `docs/ENGINEERING.md` Performance Contract section.

**Desired Outcome** — `latency-report.json` and `docs/ENGINEERING.md` agree on exactly which budgets are tracked, and only the budgets that map to real user-perceived main-path behavior (hotkey show/hide, keystroke-to-paint, diagnostics export time) are kept as release gates — not an ever-growing list of micro-metrics.

**Acceptance** — **Met (Phase 19):** ENGINEERING/QA aligned; `LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh`; RC ceilings 1000/60 ms; aspirational 50/80/30 ms documented separately.

**Risk** — Removing budgets that turn out to matter later would be a regression in observability; keep the removed/downgraded budgets recorded in `docs/DECISIONS.md` as a deliberate, reversible choice rather than silently dropping them.

**Dependencies** — P0.2/P1.5 (the hotkey latency fix and cache/repaint separation work should be done first so the "critical" budget list reflects post-fix reality, not the current 8.3 s baseline).

**Non-goals** — Do not add new performance budgets beyond what's needed for hotkey/keystroke/diagnostics-export; do not build a performance dashboard or historical trend UI.

---

### Phase 20 — P3.4 release hardening (2026-07-07)

| Deliverable | Status |
|-------------|--------|
| `docs/QA.md` § Release Candidate Gate | ✅ |
| `scripts/run_release_gate.sh` | ✅ |
| `P3_RELEASE_HARDENING_REPORT.md` | ✅ |

**P3.4 verdict:** **Go** — Phase 21 automated gate green (`P3_EXIT_SUMMARY.md`); RC pending manual supplement step 9.

### Phase 21 — P3 exit / RC decision (2026-07-07)

| Deliverable | Status |
|-------------|--------|
| `./scripts/run_release_gate.sh` (full gate) | ✅ automated steps 2–8 |
| `P3_EXIT_SUMMARY.md` | ✅ |
| Manual supplement (RC step 9) | ⏳ operator — not recorded in exit run |
| `LUMA_QA=1` fresh latency session | ⏳ optional before RC tag (stale report within budget) |

**P3 Exit verdict:** **Go** — see `P3_EXIT_SUMMARY.md`.  
**RC verdict:** **No-Go** until manual supplement recorded.

**Gate fix (Phase 21):** `scripts/run_release_gate.sh` `count_ips` — tolerate empty `Luma*.ips` glob under `set -euo pipefail`.

### P3.4 Real Smoke Test — ✅ Complete (Phase 20)

**Problem / 现象** — Every test suite referenced across Phase 0-6 (`LumaAppTests`, `LumaCoreTests`, `LumaModulesTests`, `LauncherFlowHarness`) runs via SwiftPM and does not exercise `AppCoordinator.start()`, the signed `.app` bundle, Carbon hotkey registration, the LaunchAgent, or TCC/permission prompts (S-026, C-TEST-001/004). This is the single most repeated caveat across all six prior phases.

**Evidence / 证据来源** — `CONTRACTS.md` C-TEST-001 Current Known Deviation ("No located end-to-end test for `AppCoordinator.start()` startup..."); `USABILITY_TRIAGE.md` S-026 and Test Coverage Gaps section; `MVP_SCOPE.md` Automated Acceptance Checklist note ("green automated tests are necessary but not sufficient"); confirmed presence of `scripts/verify_manual_qa.sh`, `scripts/measure_cold_start.sh`, `scripts/run_recorded_review.sh` in this phase's direct `ls scripts/`.

**Owner Area** — `scripts/build_app.sh`, `scripts/verify_manual_qa.sh`, `scripts/measure_cold_start.sh`, `docs/QA.md` release checklist.

**Desired Outcome** — A repeatable, mostly-automated smoke pass exists that runs against the real signed app (not SwiftPM alone) and covers startup, hotkey, input, results, Return/action, diagnostics, and quit — becoming a required release gate.

**Acceptance** — **Met (Phase 20):** `docs/QA.md` § Release Candidate Gate; `./scripts/run_release_gate.sh` orchestrates build/test/scanners/smokes; manual supplement documented; no UI automation framework.

**Risk** — Full automation of signed-app UI interaction (simulating Cmd+Space, typing, clicking) may require additional tooling (e.g. AX-driven UI scripting) that does not exist yet; where full automation is impractical, an explicit, precise manual checklist is an acceptable fallback — do not block P3 exit on 100% automation.

**Dependencies** — P0 fully exited (the smoke test's whole purpose is verifying P0's exit criteria); P3.2 (test reorganization) and P3.3 (performance budgets) feed into what this smoke test checks.

**Non-goals** — Do not build a full UI test-automation framework from scratch; do not require this smoke test to cover parked modules (Media, Secrets, WindowLayouts, BrowserTabs, Windows, complex Workbench); do not make this test a substitute for `swift test` — both are required, neither is sufficient alone.

## 9. Dependency Graph

- **P0.1** (Signed App Runtime / Crash Stop) blocks all runtime validation for every other P0-P3 item; nothing that says "verify on signed app" is meaningful until this closes.
- **P0.8** (Diagnostics / Doctor / Export) blocks effective debugging of every subsequent task; sequence it early within P0, not last, even though it is numbered eighth.
- **P0.2** (Hotkey) and **P0.3** (Input Responsiveness) block user-perceived usability validation for P0.4-P0.7 (Apps/Clipboard/Notes/Settings all require a summonable, responsive panel to test).
- **P0.4-P0.7** (Apps/Clipboard/Notes/Settings) are largely independent of each other once P0.1-P0.3 close, and can proceed in parallel.
- **P1** (state-owner, detail-lifecycle, MainActor-boundary, cache/repaint work) must not precede full P0 exit — P1 touches the exact code implicated in the current crashes and latency regression, and is unverifiable without a stable, diagnosable runtime.
- **P1.1** (LauncherRootController boundary) and **P1.2** (session state owner) touch overlapping files and should be sequenced together or with P1.1 informing P1.2's target ownership model.
- **P1.4** (Task/MainActor cleanup) should follow P1.1 to reduce merge conflict risk, but can run in parallel with P1.3 (Detail Lifecycle).
- **P1.5** (Cache/repaint separation) depends on P0.2's hotkey fix landing first, since P0.2 may already resolve most of the underlying cause.
- **P2** (module governance) — **complete** (`8539007c`, `P2_EXIT_SUMMARY.md`). P3 (docs/tests alignment) may proceed; do not add new P2 slices without a new planning phase.
- **P2.1–P2.5** — all delivered in Phase 15; exit gate green in Phase 16.
- **P3** (docs/tests alignment) depends on P0-P2 being substantially settled, since it documents and locks in their outcome; doing P3 early would require rework.
- **P3.3** (performance budgets) — **complete** (Phase 19, `P3_PERFORMANCE_BUDGETS_REPORT.md`).
- **P3.4** (release hardening) — **complete** (Phase 20, `P3_RELEASE_HARDENING_REPORT.md`).
- **P3 exit** — **Achieved 2026-07-07** (`P3_EXIT_SUMMARY.md`); RC tag pending manual supplement.

## 10. Release Gates

**P0 Exit** — **Achieved 2026-07-07** (`889ebd35`, Phase 9.8 Go). Re-verify via `docs/QA.md` § P0 MVP Smoke Gate on every post-P0 PR.

- A signed Luma process starts from clean state and stays running unattended.
- Cmd+Space (or documented fallback via menu bar Show) reliably shows/hides the panel within the MVP emergency ceiling.
- Apps, Clipboard, Notes, Settings, and Diagnostics main paths are each confirmed working on the signed app, not only via SwiftPM tests.
- No new `~/Library/Logs/DiagnosticReports/Luma-*.ips` appears during a full smoke pass.
- A recovery entry reaches doctor/export-diagnostics on a default install, and `diagnostics.json` is populated with real field values.
- The `docs/QA.md` P0 MVP Smoke Gate passes.

**P1 Exit** — **Entry requires P0 gate green** (see §4.5).
- `LauncherRootController`'s responsibility list is written down and narrower than today's implicit scope.
- Query/selection/content-mode/visibility/detail-lifecycle each have one documented, enforced owner with the specific C-UI-001/003/004 deviations resolved.
- MVP-path `@objc`-in-`@MainActor` scanner findings are at zero; `Task { @MainActor }` sites in the highest-traffic files are confirmed to be legitimate boundary bridges.
- Cache refresh (Open Apps, per-module warm caches, `QuerySnapshotCache`) is demonstrably decoupled from UI repaint on the query hot path.

**P2 Exit** — **Achieved 2026-07-07** (`8539007c`, `P2_EXIT_SUMMARY.md`). Re-verify via `docs/QA.md` § P0 MVP Smoke Gate + `./scripts/run_p0_smokes.sh` on release candidates.

- The MVP Core P0 modules conform to one documented lifecycle contract with no undocumented exceptions; Core P1 / conditional candidates are gated by the same contract before activation.
- Diagnostic behavior (permission/cold-cache/onboarding/timeout/empty) is consistent and documented for the MVP module set.
- `docs/PERMISSIONS.md` defaults, `docs/MODULES.md`, manifests, and `ModuleWarmupDefaults` all agree; the Windows manifest flag no longer misrepresents an unregistered module.
- Parked modules (Media, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench) have their existing re-entry criteria from `MVP_SCOPE.md` intact and unchanged.

**P3 Exit** — **Achieved 2026-07-07** (`4b34fe09` + P3.3–P3.4 docs/scripts, `P3_EXIT_SUMMARY.md`). Re-verify via `docs/QA.md` § Release Candidate Gate + `./scripts/run_release_gate.sh` on release candidates.

- No remaining doc/code mismatch from `CONTRACTS.md`'s Current Known Deviations list is doc-only and unresolved (harness #13 explicitly labeled deferred).
- Tests and QA checklists are organized around MVP main flows; `LauncherFlowHarness` production-divergence status is explicitly labeled (P3.2).
- Performance release-gating budgets (hotkey ≤ 1000 ms, keystroke ≤ 60 ms) documented in `docs/ENGINEERING.md` and `docs/QA.md` § Performance Gate; `latency-report.json` fields aligned; aspirational 50/80/30 ms non-blocking.
- Signed-app smoke test (`run_p0_smokes.sh`) referenced as mandatory release gate; `run_release_gate.sh` orchestrates automated RC steps 2–8.

**RC tag** — Requires manual supplement (RC gate step 9) recorded; see `P3_EXIT_SUMMARY.md` §9.

## 11. Risks and Non-Goals

**Risks**

- P0 crash/latency fixes may expose deeper signing, entitlement, TCC, or LaunchAgent problems beyond pure Swift code — these are outside this plan's ability to fully scope in advance and may extend P0's timeline.
- The AppKit executor-boundary crash class may not be fully explained by the two files with known compiler warnings (`ClipboardDetailView.swift`, `LauncherListView.swift`); fixing those two files first and re-measuring is the right sequence, but there is a real chance the `.ips` root cause lies elsewhere (e.g. `NotesMindMapView`, `LauncherHomeGuidePane`, both confirmed present in crash frames).
- If diagnostics (P0.8) is not fixed early and thoroughly, every subsequent P1/P2/P3 regression will continue to be undiagnosable, repeating the exact "basic functions do not work, and we can't tell why" pattern that motivated this whole investigation.
- P1 core-complexity work carries the highest regression risk in this entire plan because it touches `LauncherRootController` and its collaborators directly — the same code implicated in the current crashes. If P1 is rushed or started before P0 is fully stable and diagnosable, it risks reproducing the exact "几轮审查让项目越来越乱" pattern the user already reported.
- Scope creep is the single biggest risk to this plan's credibility: any task that starts as "fix the specific named deviation" and drifts into "while we're here, let's also improve X" should be stopped and re-scoped, per the explicit Non-goals attached to every task above.

**Non-Goals (whole-plan)**

- Do not rewrite the entire app or any entire layer (`LumaApp`/`LumaCore`/`LumaModules`/`LumaServices`/`LumaInfrastructure`).
- Do not delete source code for any parked/deferred module (Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench/Capture, Commands user scripts) — source stays retained per `MVP_SCOPE.md`'s "parked, not deleted" principle.
- Do not implement complex Workbench/Capture flows, Media/Secrets/WindowLayouts/BrowserTabs default-path features, or Windows module registration as part of any P0-P3 task in this plan.
- Do not rewrite all historical documentation in one pass — `docs/DECISIONS.md`'s decision log stays intact; only confirmed mismatches (staleness, path errors, location errors) are corrected.
- Do not treat a fully green `swift test` run as sufficient evidence for any P0/P1 exit gate — signed-app, real-runtime verification is required per C-TEST-001/004 and the repeated S-026 lesson.
- Do not reopen or re-decide `MVP_SCOPE.md`'s remaining Open Decisions (Todo default-on/deferred, Projects minimal-path scope, hotkey emergency ceiling, Clipboard history cleanup scope) inside this plan. Phase 8.5 already calibrates Translate/Snippets/Quicklinks as Core P1 candidates and Diagnostics recovery as reachable outside Commands gating.

## 12. Open Decisions Requiring User Confirmation

These are carried forward from `MVP_SCOPE.md` (not re-litigated here) but are directly load-bearing for specific tasks in this plan:

- **P0.8 mechanism resolved for Phase 9 gating**: recovery built-ins (doctor, export-diagnostics, settings recovery as needed) must be reachable independent of Commands' default-off state through menu bar or Settings. Commands may keep mirrored built-ins, but cannot be the only path; user scripts remain parked/default-off.
- **P0.2 acceptance ceiling**: confirm the 1 s emergency MVP ceiling for hotkey show (vs. holding out for the 50/80 ms contract ceiling) as the P0 gate, per `MVP_SCOPE.md` Open Decision 6.
- **P1.2 `LauncherContentMode` location**: should the type move to `LauncherContentCoordinator` to match docs, or should docs be corrected to name `LauncherKeyRouter.swift`? Either is acceptable; this plan does not prescribe which.
- **P1.2 menu-bar Show bypass**: should the bypass of `showFromCarbonHotkey()`'s guard be removed (routed through the same guarded path) or kept and proven safe with tests? Both are valid engineering outcomes for closing C-UI-001/006; the choice affects P0.2's fallback-path acceptance criteria and should be made consciously, not by default.
