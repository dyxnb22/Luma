# Phase 8 Review Summary

## 1. Executive Verdict

**No-Go for Phase 9 until three small document/decision fixes are made.**

Phase 4-7 are mostly coherent and are explicitly trying to prevent a rewrite or MVP expansion. The risk is not that the docs demand a whole-project rewrite; the risk is that Phase 9 implementers could read a few acceptance/checklist lines as permission to pull parked modules, Commands user scripts, or broad AppKit cleanup into the first implementation slice.

Minimum changes before Phase 9:

1. Tighten `docs/QA.md` / Phase 9 acceptance language so P0 smoke covers only Launch, hotkey/menu fallback, Launcher input, Apps, Clipboard, Notes, Settings, Diagnostics, permissions, logs, and config failure. Parked module smoke must be labeled post-MVP or optional.
2. Decide the Diagnostics/Doctor recovery mechanism before implementation starts: menu-bar/Settings recovery entry, Commands built-ins split, or another minimal recovery surface. Do not make full Commands/user scripts P0.
3. Clarify that Snippets, Quicklinks, Translate, and Todo are not P0 gates unless the user explicitly decides otherwise; default-on/current-on is not the same as P0.

Overall Phase 8 risk judgment: **medium**. The contract and MVP docs are directionally sound, but the implementation roadmap needs a sharper first slice and a few guardrails against scope creep.

## 2. Review Inputs

Read in full:

- `CONTRACTS.md`
- `PRODUCT_FLOWS.md`
- `MVP_SCOPE.md`
- `REFACTOR_PLAN.md`
- `CURRENT_STATE.md`
- `USABILITY_TRIAGE.md`
- `ARCHITECTURE_MAP.md`
- `MODULE_MATRIX.md`
- `Package.swift`
- `docs/ENGINEERING.md`
- `docs/MODULES.md`
- `docs/PERMISSIONS.md`

Additional docs read because Phase 4-7 cite them heavily:

- `docs/QA.md`
- `docs/DECISIONS.md`
- `docs/swift6-appkit-boundaries.md`

Source areas inspected read-only:

- `Sources/LumaModules/ModuleRegistry.swift`
- `Sources/LumaCore/Modules/WarmupTier.swift`
- `Sources/LumaModules/BuiltInModules.swift`
- `Sources/LumaModules/Commands/CommandsModule.swift`
- `Sources/LumaModules/Windows/WindowsModule.swift`
- `Sources/LumaApp/Infrastructure/AppHostService.swift`
- `Sources/LumaCore/Util/DiagnosticsExport.swift`
- `Sources/LumaInfrastructure/CrashLogBuffer.swift`
- `Sources/LumaApp/App/Hotkey/HotkeyConfig.swift`
- `Sources/LumaApp/App/MenuBarController.swift`
- `Sources/LumaApp/Settings/SettingsWindowController.swift`
- `Sources/LumaApp/Settings/SettingsSwiftUIView.swift`

Tests/scripts inventoried read-only:

- `Tests/LumaAppTests/Flow/LauncherFlowHarness.swift`
- `Tests/LumaAppTests/Flow/StabilizationFlowTests.swift`
- `Tests/LumaModulesTests/CommandModulePayloadTests.swift`
- `Tests/LumaModulesTests/CommandsModulePerformTests.swift`
- `Tests/LumaModulesTests/CommandsModuleDoctorTests.swift`
- `Tests/LumaModulesTests/BuiltInModulesTests.swift`
- `Tests/LumaModulesTests/ModuleRegistryTests.swift`
- `Tests/LumaCoreTests/DiagnosticsExportRedactionTests.swift`
- `Tests/LumaInfrastructureTests/CrashLogRedactionAtWriteTests.swift`
- `scripts/build_app.sh`
- `scripts/measure_cold_start.sh`
- `scripts/verify_manual_qa.sh`
- `scripts/run_recorded_review.sh`
- `scripts/qa/run_full_smoke.sh`
- `scripts/scan_appkit_executor_risk.sh`

No source, tests, scripts, or existing Phase documents were modified. No git add or commit was run.

## 3. Findings Requiring Changes Before Phase 9

- ID: P8-MUST-001
- Severity: High
- Evidence Level: Doc Fact + Code Fact
- Affected Docs: `docs/QA.md`, `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: `docs/QA.md` labels "Each MVP module bare prefix + Return opens detail" and includes Wordbook, Projects, Window Layouts, Media, and Secrets. `MVP_SCOPE.md` parks Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench/Capture, and Commands user scripts. Code also keeps many of these default-off, e.g. manifests and `ModuleWarmupDefaults.expertDefaultOffModuleIDs`.
- Why It Matters: A Phase 9 implementer could treat parked-module detail stability as a P0 gate and start fixing parked AppKit surfaces before Launch/hotkey/Diagnostics. That would violate the MVP contraction rule.
- Recommendation: **Must Fix Before Phase 9.** Rewrite the QA smoke wording so P0 smoke names only the MVP main path. Move parked-module detail smoke to "post-MVP / optional broad regression" and explicitly say failures there must not block P0 unless they crash the default path.

- ID: P8-MUST-002
- Severity: High
- Evidence Level: Doc Fact + Code Fact
- Affected Docs: `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: `MVP_SCOPE.md` says Diagnostics/Doctor must be reachable on default install, but `CommandsModule.manifest.defaultEnabled` is `false`, and `export-diagnostics`/`doctor` are implemented inside `CommandsModule`. `REFACTOR_PLAN.md` says P0.8 should be early but still lists it after P0.1-P0.7.
- Why It Matters: Phase 9 cannot implement recovery without choosing a mechanism. If this is left ambiguous, the fastest but wrong implementation path is making all Commands default-on, which would pull user scripts into MVP.
- Recommendation: **Must Fix Before Phase 9.** Add a one-paragraph Phase 9 precondition: recovery built-ins must be split from Commands user scripts, or exposed through a menu-bar/Settings recovery entry. Full Commands/user scripts remain parked.

- ID: P8-MUST-003
- Severity: Medium
- Evidence Level: Doc Fact + Inference
- Affected Docs: `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: `MVP_SCOPE.md` correctly marks Snippets, Quicklinks, and Translate as Core P1, but also lists them in the "MVP default-on target." Todo is conditional. `docs/MODULES.md` and code still have the seven D-012 default-on modules.
- Why It Matters: Default-on/current-on can be misread as P0-required. The user explicitly asked not to sneak Snippets / Quicklinks / Translate / Todo back into P0.
- Recommendation: **Must Fix Before Phase 9.** Add a clear line near the Phase 9 entry criteria: P0 validates Apps/Clipboard/Notes plus Settings/Diagnostics recovery; Snippets/Quicklinks/Translate are Core P1 validation only; Todo awaits user decision.

- ID: P8-MUST-004
- Severity: Medium
- Evidence Level: Doc Fact + Code Fact
- Affected Docs: `REFACTOR_PLAN.md`, `MVP_SCOPE.md`
- Evidence: Phase 7 P2.3 says to correct `WindowsModule.defaultEnabled` to `false`. Code fact: `WindowsModule` is not registered in `ModuleRegistry.allBundles` and only appears in `BuiltInModules.makeDeferred()`, but its manifest says `defaultEnabled: true`.
- Why It Matters: The doc describes this as a low-risk one-line manifest correction. That is true for runtime behavior, but it is still a code/default metadata change. Phase 9 should not start there unless the user explicitly chooses a docs/default-cleanup slice; it does not restore usability.
- Recommendation: **Must Fix Before Phase 9.** Mark Windows manifest correction as P2 docs/default hygiene, not a Phase 9 opening slice. Keep Windows unregistered and parked.

## 4. Findings That Can Defer

- ID: P8-DEF-001
- Severity: Medium
- Evidence Level: Code Fact + Doc Fact
- Affected Docs: `CONTRACTS.md`, `REFACTOR_PLAN.md`
- Evidence: `handle()` memory-only has no generic enforcement; only per-module proxy tests exist. `WindowsModule.handle` calls `CGWindowListCopyWindowInfo`, but Windows is deferred/unregistered.
- Why It Matters: Generic enforcement would be useful, but building a broad static analyzer now can balloon.
- Recommendation: **Can Defer.** For Phase 9, verify memory-only only for Apps, Clipboard, Notes, and Diagnostics-adjacent commands. Leave parked modules recorded.

- ID: P8-DEF-002
- Severity: Medium
- Evidence Level: Code Fact + Doc Fact
- Affected Docs: `CONTRACTS.md`, `REFACTOR_PLAN.md`
- Evidence: Diagnostic/corruption visibility is inconsistent across `JSONConfigPersistence`, `JSONFileStore`, and `ClipboardHistoryStore`.
- Why It Matters: Full persistence unification is not necessary for restoring usability.
- Recommendation: **Can Defer.** P0 should make doctor/export honest and reachable. Full quarantine unification belongs to P2.

- ID: P8-DEF-003
- Severity: Medium
- Evidence Level: Code Fact + Inference
- Affected Docs: `REFACTOR_PLAN.md`
- Evidence: P1.4 notes 89 warn-only AppKit target/action findings and 89 `Task { @MainActor }` sites. Parked modules account for some of that surface.
- Why It Matters: Fixing all warnings can become a broad AppKit campaign.
- Recommendation: **Can Defer.** Phase 9 should focus on crash frames and MVP-path files only. Parked-module executor warnings remain backlog unless they affect the default path.

- ID: P8-DEF-004
- Severity: Low
- Evidence Level: Doc Fact
- Affected Docs: `docs/ENGINEERING.md`, `CONTRACTS.md`, `REFACTOR_PLAN.md`
- Evidence: `LauncherContentMode` location mismatch is documented. Live ownership still belongs to `LauncherContentCoordinator`; the enum lives in `LauncherKeyRouter.swift`.
- Why It Matters: This is primarily wording/location drift, not a runtime blocker.
- Recommendation: **Can Defer.** Correct docs after P1.2 decision; do not move code just to satisfy wording.

- ID: P8-DEF-005
- Severity: Low
- Evidence Level: Doc Fact + Code Fact
- Affected Docs: `docs/PERMISSIONS.md`, `REFACTOR_PLAN.md`
- Evidence: `docs/PERMISSIONS.md` default column is stale and contradicts code/default decisions.
- Why It Matters: It is misleading, but the MVP path can still be implemented if Phase 9 ignores this stale table.
- Recommendation: **Can Defer, but cheap to fix.** If any doc cleanup happens before Phase 9, include this table. Do not let this become a module-default redesign.

## 5. Findings To Reject

- ID: P8-REJ-001
- Severity: High
- Evidence Level: Opinion + Doc Fact
- Affected Docs: `REFACTOR_PLAN.md`, `MVP_SCOPE.md`
- Evidence: Several documents list parked modules with re-entry criteria, but also say they must not block MVP.
- Why It Matters: A reviewer might propose fixing every module with AppKit warnings or detail views before P0.
- Recommendation: **Reject.** Do not fix all modules before Phase 9. Parked modules stay parked.

- ID: P8-REJ-002
- Severity: High
- Evidence Level: Code Fact + Doc Fact
- Affected Docs: `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: Diagnostics recovery is behind default-off Commands, but Commands also owns user scripts.
- Why It Matters: Making Commands fully default-on would pull local script execution into the default path.
- Recommendation: **Reject.** Do not make full Commands/user scripts P0. Split recovery built-ins or add a separate recovery entry.

- ID: P8-REJ-003
- Severity: Medium
- Evidence Level: Doc Fact + Opinion
- Affected Docs: `REFACTOR_PLAN.md`
- Evidence: P1.1/P1.2 discuss `LauncherRootController` responsibility narrowing and session state ownership.
- Why It Matters: This could be misread as "split the controller before restoring runtime."
- Recommendation: **Reject.** Do not start Phase 9 with `LauncherRootController` extraction. Only make narrow fixes required by Launch/hotkey/input recovery.

- ID: P8-REJ-004
- Severity: Medium
- Evidence Level: Doc Fact + Opinion
- Affected Docs: `REFACTOR_PLAN.md`
- Evidence: P3.3 suggests aligning performance budgets and possibly removing/instrumenting budget fields.
- Why It Matters: Budget taxonomy cleanup can distract from the confirmed 8.3s hotkey problem.
- Recommendation: **Reject for early Phase 9.** Keep hotkey p95 and keystroke p95 as gates. Budget-table cleanup waits until after runtime recovery.

## 6. Needs User Decision

- ID: P8-DEC-001
- Severity: High
- Evidence Level: Code Fact + Doc Fact
- Affected Docs: `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: `CommandsModule` is default-off, but `doctor`, `export-diagnostics`, and command-driven `settings` live there.
- Why It Matters: Recovery must be reachable without enabling a scripting module.
- Recommendation: **Needs User Decision.** Choose one: menu-bar/Settings Diagnostics entry, split recovery built-ins from user scripts, or default-on built-ins-only Commands mode.

- ID: P8-DEC-002
- Severity: Medium
- Evidence Level: Doc Fact
- Affected Docs: `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: Todo is currently default-on and uses EventKit. MVP docs call it conditional.
- Why It Matters: Todo is the only default-on candidate with a higher-sensitivity permission surprise risk.
- Recommendation: **Needs User Decision.** Keep Todo default-on Core P1 if denial UX is acceptable; otherwise defer it without deleting source.

- ID: P8-DEC-003
- Severity: Medium
- Evidence Level: Doc Fact
- Affected Docs: `MVP_SCOPE.md`, `REFACTOR_PLAN.md`
- Evidence: Hotkey P0 acceptance uses a 1s emergency ceiling while the engineering contract remains 50/80ms.
- Why It Matters: Phase 9 needs a shippable P0 bar and a long-term target.
- Recommendation: **Needs User Decision.** Confirm 1s as temporary P0 gate, with 50/80ms retained as non-P0 stabilization target.

- ID: P8-DEC-004
- Severity: Medium
- Evidence Level: Doc Fact + Code Fact
- Affected Docs: `REFACTOR_PLAN.md`, `docs/ENGINEERING.md`
- Evidence: Menu bar Show calls `show()` directly; Carbon path calls `showFromCarbonHotkey()`.
- Why It Matters: The bypass may be safe as fallback, but implementation choice affects hotkey/menu sequencing.
- Recommendation: **Needs User Decision.** Keep bypass and verify rapid sequencing, or route through a shared guarded show API that still works when hotkey registration fails.

- ID: P8-DEC-005
- Severity: Low
- Evidence Level: Doc Fact
- Affected Docs: `MVP_SCOPE.md`
- Evidence: Projects/CurrentProject minimal path is recommended only for Snippets/Quicklinks template variables.
- Why It Matters: Complex Workbench/Capture should not re-enter MVP accidentally.
- Recommendation: **Needs User Decision.** Confirm minimal CurrentProject support only; park complex Workbench/Capture.

## 7. Scope Creep / Big-Bang Risks

- Risk: "LauncherRootController Boundary" can become a broad controller rewrite.
  - Evidence Level: Doc Fact + Inference
  - Recommendation: **Can Defer.** Convert to verification-only for Phase 9 unless a specific P0 bug requires a local change.

- Risk: "Task / MainActor Boundary Cleanup" can become a 89-site mechanical sweep.
  - Evidence Level: Doc Fact
  - Recommendation: **Can Defer.** Phase 9 should fix crash-linked and MVP-path sites only; no parked-module cleanup unless default-path crash evidence appears.

- Risk: "Unified Module Lifecycle Contract" can touch every module.
  - Evidence Level: Doc Fact
  - Recommendation: **Can Defer.** Limit checks to Apps, Clipboard, Notes, and recovery built-ins for P0. Snippets/Quicklinks/Translate/Todo only when validating Core P1 or if kept default-on by decision.

- Risk: "Unified Diagnostic Behavior" can become a new taxonomy framework.
  - Evidence Level: Doc Fact + Opinion
  - Recommendation: **Can Defer.** Use existing `ModuleDiagnosticResults`/`PermissionResultBuilder`; do not design a new diagnostic system in Phase 9.

- Risk: `docs/PERMISSIONS.md` stale defaults can invite a default-state redesign.
  - Evidence Level: Code Fact + Doc Fact
  - Recommendation: **Reject.** Correct the table to match code/D-012. Do not use the table to re-enable parked modules.

- Risk: Real smoke test can become a full UI automation framework.
  - Evidence Level: Doc Fact + Opinion
  - Recommendation: **Can Defer.** A precise manual/scripted smoke is enough for Phase 9; automate only where tooling already exists.

## 8. Features To Keep Deferred / Disabled

Keep parked/default-off unless the user explicitly changes scope:

- Media / Records
- Wordbook
- Secrets
- WindowLayouts
- MenuItems / Menu Bar Search
- KillProcess
- BrowserTabs
- Windows
- Projects complex scan/manage path
- Complex Workbench / Capture / CurrentProject flows
- Commands user scripts

Keep disabled/deferred rationale:

- Evidence Level: Code Fact + Doc Fact
- Recommendation: **Reject** any Phase 9 task that makes these default-on, adds them to P0 smoke, registers Windows, or treats their detail-view warnings as P0 blockers.

Allowed minimal exceptions:

- Settings may expose module toggles.
- Diagnostics recovery may reuse Commands implementation internally, but must not expose user scripts as default P0.
- Minimal CurrentProjectService may remain available for template variables if the user confirms.

## 9. Main Path Coverage Check

- app 启动: Covered in `PRODUCT_FLOWS.md` F1 and `MVP_SCOPE.md`; still lacks true end-to-end signed-app test. Evidence Level: Doc Fact. Recommendation: **Must Fix Before Phase 9** via smoke gate.
- signed runtime: Covered in MVP/Refactor P0.1. Evidence Level: Doc Fact. Recommendation: **Must Fix Before Phase 9** as first implementation slice.
- hotkey show/hide: Covered in F2/F3/F12 and P0.2. Evidence Level: Code Fact + Doc Fact. Recommendation: **Must Fix Before Phase 9** only for acceptance wording/user decision on 1s ceiling.
- menu bar fallback: Covered, but bypass behavior needs conscious decision. Evidence Level: Code Fact. Recommendation: **Needs User Decision**.
- Launcher 输入: Covered in F5/P0.3. Evidence Level: Doc Fact. Recommendation: **Can Defer** broader session-owner refactor; keep real-app verification P0.
- 空 query 首页: Covered in F4. Evidence Level: Doc Fact. Recommendation: **Can Defer** if smoke verifies no blank/freeze.
- global search: Covered in F6 and code `ModuleRegistry.globalSearchModuleIDs`. Evidence Level: Code Fact. Recommendation: **Can Defer** broad ranker work; verify Apps/Clipboard path.
- targeted search: Covered in F7. Evidence Level: Code Fact + Doc Fact. Recommendation: **Can Defer** broad taxonomy; verify P0 modules.
- Apps search/open: Covered in MVP P0. Evidence Level: Doc Fact. Recommendation: **Must Fix Before Phase 9** as implementation slice.
- Clipboard search/copy: Covered in MVP P0. Evidence Level: Doc Fact. Recommendation: **Must Fix Before Phase 9** as implementation slice.
- Notes open/create: Covered in MVP P0. Evidence Level: Doc Fact. Recommendation: **Must Fix Before Phase 9** as implementation slice.
- Settings open/save: Covered, menu-bar Settings independent of Commands exists in code. Evidence Level: Code Fact. Recommendation: **Must Fix Before Phase 9** to verify persistence and hotkey no-op UI honesty.
- Diagnostics/Doctor/export: Covered but mechanism unresolved. Evidence Level: Code Fact + Doc Fact. Recommendation: **Needs User Decision** and **Must Fix Before Phase 9**.
- permission failure: Covered in F13 and QA, but some row-call enumeration remains incomplete. Evidence Level: Doc Fact. Recommendation: **Can Defer** full enumeration; verify MVP surfaces.
- cold cache: Covered in F14. Evidence Level: Doc Fact. Recommendation: **Can Defer** broad unification; verify Apps/Clipboard/Notes rows.
- config corruption: Covered in F15. Evidence Level: Code Fact + Doc Fact. Recommendation: **Can Defer** full store unification; doctor must be honest/reachable.
- crash/log collection: Covered in F16/MVP. Evidence Level: Code Fact. Recommendation: **Must Fix Before Phase 9** path wording and smoke evidence.
- smoke test: Covered in P3.4 and scripts exist. Evidence Level: Code Fact + Doc Fact. Recommendation: **Must Fix Before Phase 9** as a lightweight signed-app smoke checklist, not full automation.

## 10. Final Recommendation

**No-Go for Phase 9 until the three pre-Phase-9 guardrails are clarified.** After that, Phase 9 can start without broad re-planning.

Recommended first three Phase 9 implementation slices:

1. **Signed runtime + crash/log baseline:** `./scripts/build_app.sh`, real `Luma.app/Contents/MacOS/Luma` process, no new `.ips`, menu bar visible, crash-log path verified.
2. **Recovery reachability:** make Diagnostics/Doctor/export reachable without enabling Commands user scripts; populate exported diagnostics enough to be useful; document `crash-log.txt` actual path.
3. **Primary launcher loop:** hotkey/menu Show, Launcher input, `app safari` Return, `clip` search/copy, bare `n`/`n new`, Settings open/save, plus a lightweight signed-app smoke checklist.

Documents to micro-adjust before Phase 9:

- `docs/QA.md`: remove parked modules from "MVP module" smoke wording.
- `MVP_SCOPE.md`: clarify P0 vs Core P1/default-on candidate boundary.
- `REFACTOR_PLAN.md`: put recovery mechanism decision and P0.8 early sequencing before implementation; mark Windows manifest correction as P2 hygiene, not early usability work.
