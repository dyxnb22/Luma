# MVP Scope

## Scope

This file is the **Phase 6** product of the Luma stabilization investigation. It defines Luma's short-term usable core: which capabilities must be reliable now, which are parked/degraded/default-off, the acceptance bar, and the order in which parked features may re-enter.

- This file is **not** a code modification plan. It does not delete modules, does not change source, does not edit tests, does not flip default switches, and does not propose per-file refactor steps.
- "Deferred / default-off" means **parked, not deleted**. Source code is retained; long-term re-entry stays possible subject to re-entry criteria.
- This file does **not** replace `CONTRACTS.md`, `USABILITY_TRIAGE.md`, `docs/ENGINEERING.md`, or `docs/QA.md`. It consumes them.
- Where current fact differs from target (e.g. Commands default-off is a *current fact*, not a *target*), this file states the target separately and marks the mismatch.

## Inputs

Phase 0–5 artifacts read for this phase:

- `CURRENT_STATE.md` (Phase 0)
- `ARCHITECTURE_MAP.md` (Phase 1)
- `MODULE_MATRIX.md` (Phase 2)
- `PRODUCT_FLOWS.md` (Phase 3)
- `CONTRACTS.md` (Phase 4)
- `USABILITY_TRIAGE.md` (Phase 5)

Engineering docs read for this phase:

- `docs/ENGINEERING.md` (Performance Contract, hot-path rules, diagnostics export, lazy AX)
- `docs/MODULES.md` (MVP default-on / default-off lists, module surfaces)
- `docs/PERMISSIONS.md` (permission matrix; noted stale vs D-012)
- `docs/QA.md` (manual main-path smoke, release checklist)
- `docs/DECISIONS.md` (D-001…D-022)

No Swift source, tests, or scripts were modified. No tests were run in this phase (this phase is scope definition only).

## Decision Summary

- The current goal is to restore a **small, reliable** Luma, not to ship every module.
- MVP priority order: **launch + signed app stays running → hotkey show/hide → Apps → Clipboard → Notes → Settings → Diagnostics/Doctor**. Snippets, Quicklinks, and Translate are **Core P1 candidates**, not P0 gates. Todo follows the user decision below and is not a P0 gate until decided.
- Other modules (Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows, complex Workbench/Capture, user Commands scripts) keep source but must not block MVP and do not enter the main path on the default install.
- **Diagnostics / Doctor is a recovery capability.** It must be reachable on a default install outside Commands default-off gating; the current fact "Commands default-off cuts off `cmd doctor` / `cmd export-diagnostics`" is a mismatch to resolve in MVP scope, not a target to preserve (per rule 10: do not mistake current fact for target).
- **Phase 9 P0 slices must not depend on Core P1 or conditional modules.** Launch/hotkey/menu fallback/Launcher input/Apps/Clipboard/Notes/Settings/Diagnostics must be implementable and gateable without Snippets, Quicklinks, Translate, Todo, or any parked module.
- **Signed app / real runtime / manual QA evidence outranks SwiftPM test green.** S-026 explicitly warns that green `swift test` does not prove the signed app, hotkey, LaunchAgent, or diagnostics actually work.
- The Phase 5 hard blockers (S-025, S-002, S-020/S-021, S-026, S-024, AppKit/MainActor warnings + `.ips`) are mapped into MVP scope as must-address-in-MVP risks, not deferred.

## MVP Principles

1. **Small core first.** Maximize the chance the launch→hotkey→search→action→recover loop works before adding surfaces.
2. **No high-permission surprise on the default path.** Default-on modules must be useful without Automation/Keychain/Screen-Recording/EventKit-as-blocking. EventKit (Todo) is the one borderline case — see Open Decisions.
3. **No hidden failure.** Permission denial, cold cache, missing data source, config corruption, and read failure must surface as a row/banner/status, never silent empty (C-FAIL-001/002/003/006).
4. **No unbounded work on the hot path.** `handle` is memory-only; hotkey→visible and keystroke→paint stay within contract budgets (C-HOT-001/002/003/004).
5. **Recovery paths must be reachable.** Doctor, export-diagnostics, Settings, and menu bar Show cannot be fully cut off by default config (C-DEFAULT-005).
6. **Tests must model production paths or declare their limits.** A green SwiftPM test that does not exercise `AppCoordinator.start()` / signed app / Carbon / LaunchAgent is recorded as a coverage gap, not as proof (C-TEST-001, C-TEST-004; S-026).
7. **Deferred means parked, not deleted.** Source, manifests, and tests for parked modules stay in tree; re-entry is gated by re-entry criteria, not by deletion.

## MVP Feature Set

MVP Tier legend:

- **Core P0** — without it Luma is unusable on the main path.
- **Core P1** — main-path enhancement; may degrade gracefully.
- **Recovery P0** — not necessarily daily-use, but must be reachable for repair/diagnosis.
- **Support P1** — helps locate or explain problems.

| Feature | Include? | MVP Tier | User Value | Required Flows | Required Contracts | Current Blockers | Acceptance Criteria |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Launch / signed app stays running | Yes | Core P0 | App exists and stays alive | F1, F2, F3 | C-APPKIT-001, C-APPKIT-002, C-ASYNC-001, C-TEST-001 | S-025, AppKit/MainActor warnings, three `.ips` | `build_app.sh` succeeds; real `Luma` process stays running 10 min; no new `.ips`; menu bar icon present |
| Hotkey show/hide | Yes | Core P0 | Primary entry/exit | F2, F3, F12 | C-HOT-002, C-ASYNC-002, C-ASYNC-003, C-UI-001, C-APPKIT-002 | S-002 (p95 ≈ 8.3s), S-001, S-014 | Cmd+Space shows within short-term 1s ceiling; Esc/Cmd+Space hides; rapid toggle stable; no `.ips` |
| Menu bar Show fallback | Yes | Core P0 | Entry when hotkey fails | F3, F12 | C-UI-001, C-ASYNC-002 | S-003 (bypasses Carbon guard/debounce) | Menu bar Show opens panel; works when hotkey registration failed |
| App search and open | Yes | Core P0 | Find/launch/activate apps | F4, F5, F6, F7, F9 | C-HOT-001, C-HOT-003, C-HOT-004, C-FAIL-002, C-MODULE-002 | S-004, S-006, S-010 | `app safari` returns and activates Safari; `app top` shows warming row when cold, then rows; Return launches/focuses |
| Clipboard search/copy | Yes | Core P0 | Reuse copied text | F7, F9, F10, F13 | C-HOT-001, C-FAIL-001, C-PERSIST-001, C-CACHE-002 | S-016, 38MB history + `.corrupt-*.bak`, paste AX | `clip` opens detail; search returns rows; Return copies; paste-directly shows AX guidance when denied |
| Notes open/create | Yes | Core P0 | Open/create notes | F7, F10, F11, F15 | C-FAIL-003, C-PERSIST-001, C-DETAIL-003, C-APPKIT-001, D-016 | S-017, `NotesMindMapView.isFlipped` `.ips` frame | Bare `n` opens detail or onboarding "Choose a Notes root folder"; `n new` creates; open resolves under root; no `.ips` |
| Settings open/save | Yes | Core P0 (recovery) | Configure + recover | F9, F13, F15, F16 | C-FAIL-003, C-PERSIST-001, C-DEFAULT-005 | S-019, `HotkeyConfig.save()` no-op, Commands default-off gates `settings` entry | Menu bar Settings opens; saving a non-destructive setting persists across restart; Notes root configurable |
| Diagnostics / Doctor / export | Yes | Recovery P0 | Repair + support evidence | F7, F16 | C-DIAG-001, C-DIAG-002, C-DIAG-004, C-DEFAULT-005, C-FAIL-006 | S-020, S-021, S-024 | Recovery entry reachable outside Commands default-off gating; command built-ins may mirror it but cannot be the only path; export writes `~/Library/Logs/Luma/diagnostics.json` with populated fields |
| Permission guidance | Yes | Core P1 | Explain denial | F13 | C-FAIL-001, C-FAIL-005, C-DEFAULT-002, D-010 | S-015 | AX/EventKit/Automation denial shows actionable row/banner, never silent empty |
| Persistence / restart sanity | Yes | Core P1 | State survives restart | F1, F12, F15 | C-PERSIST-001, C-CACHE-001 | S-023, JSON read-failure silent fallback | Enabled modules + Notes root + non-destructive Settings survive restart; lost state is explainable |
| Crash / logs collection | Yes | Recovery P0 / Support P1 | Diagnose crashes | F16 | C-DIAG-003, C-DIAG-004, C-APPKIT-001 | S-022, S-025 | `crash-log.txt` path documented (`~/Library/Application Support/Luma/`); new `.ips` locatable; breadcrumbs appended on hotkey/action failure |
| Snippets copy/expand | Yes (default-on candidate) | Core P1 | Text templates | F7, F9 | C-HOT-001, C-FAIL-001, C-MODULE-002 | AX on paste only | `s <query>` searches; Return copies; exact-trigger expansion works; paste degrades with AX guidance |
| Quicklinks URL launch | Yes (default-on candidate) | Core P1 | URL templates | F7, F9 | C-HOT-003, D-017 | None (no high-sensitivity perm) | Configured exact triggers open URLs (http/https/mailto); `ql` opens manager |
| Translate | Yes (default-on candidate) | Core P1 | Translate text | F7, F9, F10 | C-HOT-001, C-FAIL-004 | None high-sensitivity (Translation framework) | `tr <text>` translates; bare `tr` opens detail; errors mapped |
| Todo | Conditional (Open Decision) | Core P1 or Deferred | Reminders | F7, F9, F13 | C-FAIL-001, C-FAIL-003, C-DEFAULT-002 | S-018, EventKit permission | If kept: `t`/`todo` lists/creates; denial shows actionable row. If deferred: source retained, re-entry on EventKit QA |

## MVP Core Details

### Launch / Signed App Runtime

**MVP Decision** — Included as Core P0.

**Why Included** — Without a running signed process, every other capability is moot. S-025 confirms no true `Luma` process is running and three same-day `.ips` reports exist.

**User Value** — User launches Luma and it stays running as an accessory-policy background process with a menu bar item.

**Required Flows** — F1 (startup), F2 (hotkey registration), F3 (summon).

**Required Contracts** — C-APPKIT-001, C-APPKIT-002, C-ASYNC-001, C-TEST-001, C-TEST-003.

**Acceptance Criteria**
- `./scripts/build_app.sh` succeeds and produces `build/Luma.app/Contents/MacOS/Luma`.
- A real `Luma.app/Contents/MacOS/Luma` process exists and stays running for ≥ 10 minutes without external intervention.
- No new `~/Library/Logs/DiagnosticReports/Luma-*.ips` appears during the run.
- Menu bar status item is present.
- If a crash does occur, the newest `.ips` faulting thread is locatable and attributable (not an orphan stack).
- LaunchAgent (`build/Luma.app/Contents/MacOS/Luma`) restarts Luma after crash per README.

**Known Blockers** — S-025 (no running process + `.ips`); AppKit/MainActor compiler warnings in `ClipboardDetailView.swift` and `LauncherListView.swift`; `.ips` faulting frames include `@objc LauncherHomeGuidePane.tableView(_:shouldSelectRow:)`, `@objc NotesMindMapView.isFlipped.getter`, `swift_getObjectType` — same AppKit executor-boundary class as C-APPKIT-001/002.

**Evidence / Tests / QA** — `swift build` (necessary, not sufficient); `swift test --filter AppKitExecutor`; `scripts/scan_appkit_executor_risk.sh`; manual: `./scripts/build_app.sh`, Activity Monitor/pgrep check, clear old `.ips` and reproduce once.

---

### Hotkey Show/Hide

**MVP Decision** — Included as Core P0.

**Why Included** — Hotkey is the primary entry; S-002 confirms p95 ≈ 8.3s vs the 50/80ms contract, which alone makes the product feel broken.

**User Value** — Cmd+Space summons the launcher quickly; Esc or Cmd+Space hides it; rapid toggle is stable.

**Required Flows** — F2 (registration), F3 (summon), F12 (hide).

**Required Contracts** — C-HOT-002, C-ASYNC-002, C-ASYNC-003, C-UI-001, C-APPKIT-002, C-TEST-001.

**Acceptance Criteria**
- Cmd+Space shows the panel within the **short-term MVP ceiling of 1 second** (contract target remains 50ms p95 / 80ms ceiling; the 1s bar is an emergency floor so S-002 does not block all MVP validation — see Open Decisions).
- Esc / Cmd+Space hides the panel within the 40ms hide ceiling.
- Rapid show/hide/show loops do not produce a stuck, transparent, or unfocused panel.
- No new `.ips` during repeated toggle.
- Hotkey registration failure is visible (menu bar warning icon) and recorded in `crash-log.txt`.

**Known Blockers** — S-002 (8.3s p95); S-001 (no exact repro yet); S-014 (hide/re-summon state); `HomeLatencyTracker.markHotkey()` is the instrumentation point; OpenAppsHomeProvider/home refresh suspected contributor (per F3/F4).

**Evidence / Tests / QA** — `HotkeyReregisterTests`, `HotkeyToggleExecutorTests`, `HotkeyDoubleFireTests`, `LauncherShowHideStateTests`, `LauncherPanelVisibilitySessionTests`; manual: signed-app Cmd+Space timing, compare Carbon vs menu bar Show, inspect `latency-report.json` after run.

---

### Menu Bar Show Fallback

**MVP Decision** — Included as Core P0 (recovery entry).

**Why Included** — When hotkey registration fails or is delayed, menu bar Show is the only remaining user-facing entry. S-003 records it bypasses `showFromCarbonHotkey()` guards.

**User Value** — A clickable menu bar path to open the launcher regardless of hotkey state.

**Required Flows** — F3, F12.

**Required Contracts** — C-UI-001, C-ASYNC-002, C-TEST-001.

**Acceptance Criteria**
- Menu bar "Show" opens the panel when the hotkey is unregistered or has failed.
- Rapid menu bar Show + Cmd+Space sequencing does not leave the panel in an inconsistent visibility state.
- The bypass of Carbon guard/debounce (current fact) does not introduce a stuck-visible or double-show state.

**Known Blockers** — S-003; the bypass is a documented code fact; no runtime failure reproduced, but the divergence from the documented two-path model is a risk.

**Evidence / Tests / QA** — Carbon show/hide tests; manual menu bar Show test against a running signed app, including the "hotkey failed" scenario.

---

### App Search/Open

**MVP Decision** — Included as Core P0.

**Why Included** — App launch/focus is the canonical Luma action; default-on, global-search fast tier, no high-sensitivity permission.

**User Value** — `app safari` opens/focuses Safari; `app top` shows memory leaders; home shows running apps.

**Required Flows** — F4, F5, F6, F7, F9, F14 (cold cache).

**Required Contracts** — C-HOT-001, C-HOT-003, C-HOT-004, C-FAIL-002, C-MODULE-002, C-CACHE-002.

**Acceptance Criteria**
- `app safari` returns Safari and Return launches/activates it within the action budget.
- `app top` shows a warming row when the memory cache is cold, then real rows (C-FAIL-002).
- Empty home left column shows running apps (warming vs truly-empty distinguished).
- No AX banner on plain home / ordinary app search (D-010).
- `app` bare shows Apps guide/rows, not blank.

**Known Blockers** — S-004, S-006, S-010 (user-reported but unconfirmed); `launcherFlowHarnessReplaysQuery` previously failed (now passing, S-026 warns harness ≠ production).

**Evidence / Tests / QA** — `AppsModuleTests`, `AppsModuleTopQueryPerformanceTests`, `AppsMemoryTopSWRTests`, `ModuleColdCacheTests`, `simulatedUserTouchesEveryRequestedFeature`, `appsModuleTopTargetedQueryStaysUnderBudget`; manual: `app safari`, `app top`, home left column.

---

### Clipboard Search/Copy

**MVP Decision** — Included as Core P0.

**Why Included** — Default-on, global-search contributing; primary "reuse copied text" surface.

**User Value** — `clip` opens history; search matches; Return copies; paste-directly when AX granted.

**Required Flows** — F7, F9, F10, F13.

**Required Contracts** — C-HOT-001, C-FAIL-001, C-PERSIST-001, C-CACHE-002, C-HOT-006.

**Acceptance Criteria**
- `clip` opens Clipboard detail.
- Search returns in-memory matches (≥3 chars, capped 3 in global search).
- Return copies the entry to the pasteboard (no AX needed for copy).
- Paste-directly shows `permissionRequired(.accessibility)` guidance when AX denied, never silent success (C-FAIL-004).
- Privacy filters skip secret-looking values; large history does not block detail open.

**Known Blockers** — S-016; `clipboard-history.json` ≈ 38MB with a sibling `.corrupt-*.bak`; separate quarantine scheme not visible to `cmd doctor` (S-024); `ClipboardDetailView.swift` AppKit/MainActor warnings (S-025 class).

**Evidence / Tests / QA** — `ClipboardPersistenceTests`, `ClipboardHistoryTests`, `ClipboardSearchPerformanceTests`, `PasteOutcomeTests`; manual: `clip`, search, Return copy, paste-directly with AX denied.

---

### Notes Open/Create

**MVP Decision** — Included as Core P0.

**Why Included** — Default-on, bare `n` opens detail; Markdown workspace is a headline Luma surface. S-017 + `.ips` `NotesMindMapView.isFlipped` frame make this a crash-adjacent risk.

**User Value** — Open notes in Typora/system editor, create notes/folders, daily note; configure root.

**Required Flows** — F7, F10, F11, F15.

**Required Contracts** — C-FAIL-003, C-PERSIST-001, C-DETAIL-003, C-APPKIT-001, D-016, C-ASYNC-002 (NotesDetailRefreshGate).

**Acceptance Criteria**
- Bare `n` opens Notes detail or, if root unset, shows onboarding "Choose a Notes root folder".
- `n new` / `n daily` create a note under the configured root and open it via `openLocalFileURL` after containment (D-016).
- Open paths resolve only under the configured root.
- Esc returns to home/results; search field is editable afterward (launcher-navigation rule).
- No new `.ips` involving `NotesMindMapView` or other Notes AppKit overrides.

**Known Blockers** — S-017; `NotesMindMapView.isFlipped.getter` in `.ips`; FSEvents async refresh staleness; `notes.json` quarantine via `JSONConfigPersistence`.

**Evidence / Tests / QA** — `NotesMetaTests`, `NotesOpenPathSecurityTests`, `NotesCaptureTests`, `NotesCreateOpensLocalFileTests`, `NotesTreeIndexTests`; manual: `n`, `n new`, root configuration via Settings, Esc round-trip.

---

### Settings Open/Save

**MVP Decision** — Included as Core P0 (recovery surface).

**Why Included** — Settings is the user-facing recovery path for modules, roots, and permissions. S-019 records uncertainty; Commands default-off also gates the `settings` command entry.

**User Value** — Open Settings from menu bar; save a non-destructive setting; configure Notes root and enabled modules; restart preserves it.

**Required Flows** — F9, F13, F15, F16.

**Required Contracts** — C-FAIL-003, C-PERSIST-001, C-DEFAULT-005, C-DEFAULT-004.

**Acceptance Criteria**
- Menu bar Settings opens the Settings window on a default install (not gated by Commands).
- Saving a non-destructive setting (e.g. Notes root, enabled modules, clipboard retention) persists across restart.
- `HotkeyConfig.save()` no-op is documented and does not mislead users (current fact, not target — UI must not advertise an editable hotkey that silently fails).
- Disabling a module tears down the actor, cancels queries, closes detail, evicts pools (per `docs/PERMISSIONS.md`).

**Known Blockers** — S-019; `HotkeyConfig.save()` no-op; Commands default-off may cut the `settings` command entry (menu bar Settings must remain independent).

**Evidence / Tests / QA** — No full end-to-end Settings test confirmed (gap); manual: menu bar Settings open, save Notes root, restart, verify persistence.

---

### Diagnostics / Doctor / Export

**MVP Decision** — Included as Recovery P0. **Must be reachable on a default install** (target, not current fact).

**Why Included** — S-020/S-021 confirm `cmd doctor` and `cmd export-diagnostics` are cut off by Commands default-off, and `diagnostics.json` is missing. Recovery and support cannot depend on the user first enabling an expert module.

**User Value** — Run doctor to see hotkey/permission/corruption/latency status; export redacted diagnostics for support.

**Required Flows** — F7, F16.

**Required Contracts** — C-DIAG-001, C-DIAG-002, C-DIAG-004, C-DEFAULT-005, C-FAIL-006, C-PERSIST-002.

**Acceptance Criteria**
- On a default install, a recovery entry (doctor + export-diagnostics) is reachable outside Commands default-off gating. A menu bar or Settings recovery entry is required; Commands built-ins may remain as mirrored command entries, but cannot be the only path.
- Doctor reports hotkey registration, corrupt config files, and latency p95.
- Export diagnostics writes `~/Library/Logs/Luma/diagnostics.json` with populated `platform`, `modules`, `permissions`, `recentErrors`, `corruptConfigFiles` (current production call site leaves fields empty — S-024 / C-DIAG-002 mismatch).
- Corruption from `JSONConfigPersistence` and clipboard `.corrupt-*.bak` is visible to doctor (currently split paths — S-024).

**Known Blockers** — S-020 (Commands default-off), S-021 (`diagnostics.json` missing), S-024 (memory-only registry + split quarantine + silent read fallback + empty payload fields).

**Evidence / Tests / QA** — `CommandsModuleDoctorTests`, `JSONConfigPersistenceTests`, diagnostics export unit path; manual: menu bar/Settings recovery entry, optional mirrored `cmd doctor` / `cmd export-diagnostics`, verify file + fields on default install.

---

### Permission Guidance

**MVP Decision** — Included as Core P1.

**Why Included** — "No hidden failure" is an MVP principle; S-015 records uncertainty about banner timing.

**User Value** — When AX/EventKit/Automation is denied, the user sees an actionable row/banner, not a blank panel.

**Required Flows** — F13.

**Required Contracts** — C-FAIL-001, C-FAIL-005, C-DEFAULT-002, D-010.

**Acceptance Criteria**
- AX banner is lazy and surface-gated: no banner on plain home / ordinary app search; banner on AX-dependent surfaces (Snippets paste, Window Layouts, Menu Items, Clipboard paste-directly).
- EventKit denial (Todo) and Automation denial (Browser Tabs) surface as in-list rows, not the AX banner.
- Denied rows are actionable (link to System Settings / permission request).

**Known Blockers** — S-015; full enumeration of `PermissionResultBuilder.row` call sites not confirmed (Phase 2 unknown).

**Evidence / Tests / QA** — `PermissionBanner` tests; manual: AX denied → Snippet paste / Window Layouts; EventKit denied → Todo; Automation denied → Browser Tabs.

---

### Persistence / Restart Sanity

**MVP Decision** — Included as Core P1.

**Why Included** — S-023 records restart-loses-state risk; JSON read failures can silently fall back.

**User Value** — Enabled modules, Notes root, and non-destructive Settings survive restart.

**Required Flows** — F1, F12, F15.

**Required Contracts** — C-PERSIST-001, C-CACHE-001, C-DEFAULT-004.

**Acceptance Criteria**
- After a normal restart, enabled modules, Notes root, and saved Settings persist.
- `launcher-resume.json` restores session sanely or is documented as best-effort.
- Lost state (if any) is explainable, not silent.

**Known Blockers** — S-023; JSON read-failure silent fallback (S-024); `ConfigCorruptionRegistry` memory-only.

**Evidence / Tests / QA** — `JSONConfigPersistenceTests`, persistence tests; manual: save settings, restart via `build_app.sh`, verify.

---

### Crash / Logs Collection

**MVP Decision** — Included as Recovery P0 / Support P1.

**Why Included** — S-022/S-025 require that crashes be locatable and breadcrumbs explainable.

**User Value** — After a crash or failed action, support can find `.ips` and `crash-log.txt`.

**Required Flows** — F16.

**Required Contracts** — C-DIAG-003, C-DIAG-004, C-APPKIT-001.

**Acceptance Criteria**
- `~/Library/Application Support/Luma/crash-log.txt` path is documented (current docs only mention `~/Library/Logs/Luma/diagnostics.json` — mismatch to fix in MVP-critical docs).
- `CrashLogBuffer.persist()` write failure is isolated but explainable (currently `try?` swallows — S-024 class).
- Hotkey registration failure and action failure append breadcrumbs.
- New `.ips` files are attributable to a faulting thread.

**Known Blockers** — S-022 (path mismatch, breadcrumbs-only), S-025 (`.ips`), `CrashLogBuffer.persist()` `try?`.

**Evidence / Tests / QA** — Crash log redaction tests; manual: trigger hotkey failure / failed action, inspect `crash-log.txt` tail and `~/Library/Logs/DiagnosticReports/`.

---

### Snippets Copy/Expand (default-on candidate)

**MVP Decision** — Included as Core P1 (default-on per D-012).

**Why Included** — Default-on; copy works without AX; paste degrades with guidance. No high-sensitivity permission.

**User Value** — `s <query>` searches; Return copies; exact triggers expand text.

**Required Flows** — F7, F9.

**Required Contracts** — C-HOT-001, C-FAIL-001, C-MODULE-002, C-HOT-006.

**Acceptance Criteria** — `s`/`snip` searches; Return copies; exact-trigger expansion works in global search; paste/insert shows AX guidance when denied; `handle` does not await AX (test-enforced).

**Known Blockers** — None high-sensitivity; excluded from `QuerySnapshotCache` by design.

**Evidence / Tests / QA** — `SnippetsStoreTests`, `SnippetIndexTests`, `snippetsHandleDoesNotAwaitAccessibility`; manual: `s`, copy, paste with AX denied.

---

### Quicklinks URL Launch (default-on candidate)

**MVP Decision** — Included as Core P1 (default-on per D-012).

**Why Included** — Default-on; no permission; URL scheme restricted (D-017). Main-path enhancement, not blocking.

**User Value** — Configured exact triggers (`gh`, `g`, …) open URLs; `ql` manages links.

**Required Flows** — F7, F9.

**Required Contracts** — C-HOT-003, D-017, C-FAIL-004.

**Acceptance Criteria** — Exact first-token triggers open http/https/mailto URLs; `ql` opens manager; deletes require confirmation; template expansion works.

**Known Blockers** — None material.

**Evidence / Tests / QA** — `QuicklinksModuleTests`, `QuicklinksTests`; manual: configured trigger, `ql`.

---

### Translate (default-on candidate)

**MVP Decision** — Included as Core P1 (default-on per D-012).

**Why Included** — Default-on; no high-sensitivity permission (Translation framework / Shortcuts); real work in `perform`/detail, `handle` only builds a row. Does not block Apps/Clipboard/Notes/Diagnostics.

**User Value** — `tr <text>` translates; bare `tr` opens detail with language chips.

**Required Flows** — F7, F9, F10.

**Required Contracts** — C-HOT-001, C-FAIL-004.

**Acceptance Criteria** — `tr <text>` translates via system/Shortcut; bare `tr` opens detail; errors mapped via `TranslationClient`; target language from config (default `en`).

**Known Blockers** — None material; depends on Translation framework availability.

**Evidence / Tests / QA** — `TranslationFailureTests`, `TranslateBareContractTests`; manual: `tr hello`, bare `tr`.

---

### Todo (conditional — Open Decision)

**MVP Decision** — **Conditional, pending product decision.** Default-on today (D-012), but it is the only default-on module requiring a high-sensitivity permission (EventKit).

**Why Included or Deferred** — If EventKit denial is reliably actionable (row + link to System Settings), Todo stays MVP Core P1. If EventKit prompts on a fresh default install create a "permission surprise" that conflicts with MVP Principle 2, Todo is deferred to parked.

**User Value** — List/create/complete reminders.

**Required Flows** — F7, F9, F13.

**Required Contracts** — C-FAIL-001, C-FAIL-003, C-DEFAULT-002.

**Acceptance Criteria (if kept)** — `t`/`todo` lists today/due when authorized; create reminder via NLP; denial shows actionable row; EventKit store-change listener lifecycle is stable.

**Re-entry Criteria (if deferred)** — EventKit denial UX verified on signed app; no permission prompt on first cold start unless user invokes Todo; actionable denial row verified.

**Known Blockers** — S-018; EventKit permission state not captured in Phase 5.

**Evidence / Tests / QA** — `TodoTimeParserTests`, `TodoModuleStoreChangesTests`, `simulatedUserTouchesEveryRequestedFeature`; manual: `t`, create, denial row.

## Deferred / Parked Features

"Deferred" = source retained, not registered on the default main path, not deleted. Re-entry is gated by the criteria below.

| Feature / Module | Current State | MVP Decision | Why Deferred | What Remains Available | Must Not Block MVP | Re-entry Criteria |
| --- | --- | --- | --- | --- | --- | --- |
| Media / Records | default-off | Parked | Personal logbook, not main path; CSV export to Downloads | Source + detail + tests retained; enable in Settings | Must not appear in default home guide; no warmup on default install | handle memory-only verified; CSV export path audited; detail flow verified; production-path smoke passes; no AppKit warnings/crashes on its surface |
| Wordbook | default-off | Parked | Large in-memory index (50k rows); CSV import after enabling | Source + detail + review + tests retained | No default warmup; no default home guide | cold due-cache row verified; search index budget verified; detail review flow verified; production-path smoke passes |
| Secrets | default-off | Parked | Keychain vault; high-sensitivity | Source + detail + tests retained; enable in Settings | No default warmup; values never in `handle` (already enforced) | unlock flow verified; auto-clear/relock verified; Keychain failure diagnostic verified; excluded from `QuerySnapshotCache` confirmed |
| Window Layouts | default-off (D-020) | Parked | AX-dependent; no warm-cache hot-path registration until warm-cache ships (D-020) | Source + presets + tests retained | No default warmup; no AX prompt on default install | AX denial row verified; warm-cache hot-path registration lands per D-020; no AppKit/AX crash on its surface |
| Menu Items / Menu Bar Search | default-off | Parked | AX-dependent; stale cache risk | Source + detail + tests retained | No default warmup; no AX prompt on default install | AX trust diagnostic verified; bounded cache refresh verified; stale-cache behavior verified; detail/action verified |
| Kill Process | default-off | Parked | Expert quit/force-kill; D-014 routes bare `quit`/`k` here | Source + tests retained; bare `quit`/`k` do not respond on default install (D-014) | Must not capture bare `quit` on default install in a way that surprises users | cold refresh row verified; guarded-bundle confirmation verified; no accidental system-app kill |
| Browser Tabs | default-off | Parked | Automation prompts sensitive; 900ms timeout | Source + adapters + tests retained | No Automation prompt on default install; AppleScript never on hot path (test-enforced) | Automation denial actionable verified; cached-tabs-only `handle` verified; first-cold-query degraded row verified |
| Windows | deferred (not registered) | Parked | `handle` calls `CGWindowListCopyWindowInfo` directly — violates memory-only; manifest `defaultEnabled: true` misleading | Source retained in `BuiltInModules.makeDeferred()` | Must not be registered in `ModuleRegistry.allBundles` until re-entry criteria met | warm-cache + tests land (`docs/MODULES.md:51`); memory-only `handle` verified; screen-recording permission diagnostic verified; manifest flag corrected |
| Projects / CurrentProject / Workbench | Projects default-off; Workbench not a module | Parked (complex Workbench/Capture fully parked; minimal Projects path optional) | Complex Workbench/Capture flows heavy; Projects needs scan-root config | Source + detail + tests retained. **Minimal path:** `CurrentProjectService` is bootstrapped in `AppCoordinator.init` and used for Snippets variable expansion and project-link template vars — this minimal path stays available so Snippets/Quicklinks variables work. Complex capture/convert flows do not enter MVP. | Complex Workbench/Capture must not stall `proj` preview or hot path (`docs/QA.md:139`); no background disk scan per query | scan-root onboarding verified; `proj` preview does not fetch selection unless attach/capture; capture/convert flows verified separately; `workbench-activity.json` quarantine visible to doctor |
| Commands user scripts | default-off | Parked (user scripts only) | Full custom script platform not MVP | Built-in recovery commands (`doctor`, `export-diagnostics`, `settings`, `exit`) are **MVP Recovery P0** and must be reachable (see Diagnostics section). User `commands.json` scripts stay parked. | User-script execution must not gate recovery built-ins | `ScriptRunnerSecurityPolicy` verified; executable/CWD allowlists verified; failure `CrashLogRecording` verified; script run does not block panel dismissal |
| Todo (alt) | default-on | See Open Decision | EventKit permission surprise risk | Source + tests retained | If deferred: no EventKit prompt on default cold start | EventKit denial UX verified; actionable row verified |

## Default-On Proposal For MVP

This is a **scope target**, not a code change. It does not edit manifests, defaults, or `ConfigurationStore`.

**MVP default-on target:**
- Apps
- Clipboard
- Notes
- Snippets — Core P1 candidate, not a P0 gate
- Quicklinks — Core P1 candidate, not a P0 gate
- Translate — Core P1 candidate, not a P0 gate
- Diagnostics/Settings recovery ability — reachable **regardless of Commands module default** (i.e. doctor/export-diagnostics/settings entry must not be fully cut off by Commands default-off)
- Todo — **conditional on Open Decision** (EventKit), not a P0 gate until decided

**MVP default-off target:**
- Media, Wordbook, Secrets, WindowLayouts, MenuItems, KillProcess, BrowserTabs, Windows
- Complex Workbench / Capture flows
- Commands **user scripts** (built-in recovery commands excluded from default-off gating)

**Current mismatches (target ≠ current fact):**
- **Commands default-off blocks diagnostics** (S-020): `cmd doctor` / `cmd export-diagnostics` unreachable on a fresh default install. Phase 8.5 decision: recovery built-ins must be reachable outside Commands default-off gating through menu bar or Settings; command entries may mirror that path but cannot be the only path.
- **`docs/PERMISSIONS.md` "Default" column stale** (Phase 2 mismatch): marks Menu Items, Wordbook, Window Layouts, Media, Projects, Kill Process, Secrets, Workbench as "on", contradicting D-012 and manifest `defaultEnabled: false`. Target: sync with D-012.
- **Windows manifest `defaultEnabled: true` misleading** (Phase 2 mismatch): module is not registered in `ModuleRegistry.allBundles`. Target: P2 hygiene only; manifest flag should eventually be consistent with deferred registration, but this must not be a Phase 9 P0 starting slice. Windows remains deferred/unregistered.
- **`diagnostics.json` payload fields empty** (S-024 / C-DIAG-002): production call site passes only `latencyP95` + breadcrumbs; `platform`/`modules`/`permissions`/`recentErrors` default empty. Target: populated fields.
- **`crash-log.txt` path undocumented** (S-022): file is at `~/Library/Application Support/Luma/crash-log.txt`, docs only mention `~/Library/Logs/Luma/`. Target: documented.
- **`HotkeyConfig.save()` no-op** (S-019): UI must not advertise an editable hotkey that silently fails. Target: UI honesty.

## MVP Acceptance Checklist

### Automated (minimum)

- [ ] `swift build`
- [ ] `swift test --filter LumaAppTests`
- [ ] `swift test --filter AppKitExecutor`
- [ ] `swift test --filter LauncherFlowHarness`
- [ ] `swift test --filter PermissionBanner`
- [ ] `swift test --filter CommandsModuleDoctor`
- [ ] `swift test --filter LumaCoreTests` (persistence, visibility session, query dispatcher)
- [ ] `swift test --filter LumaModulesTests.simulatedUserTouchesEveryRequestedFeature`
- [ ] `swift test --filter LumaModulesTests.appsModuleTopTargetedQueryStaysUnderBudget`

Note (S-026): green automated tests are necessary but not sufficient. They do not prove signed-app launch, Carbon hotkey, LaunchAgent, TCC, or diagnostics reachability.

### Manual / Runtime

- [ ] `./scripts/build_app.sh` succeeds
- [ ] A real `Luma.app/Contents/MacOS/Luma` process exists and stays running
- [ ] Cmd+Space shows the panel within the **short-term 1s MVP ceiling** (contract target 50/80ms remains; 1s is the emergency floor — see Open Decisions)
- [ ] Menu bar Show opens the panel
- [ ] Esc / Cmd+Space hides the panel
- [ ] `app safari` returns Safari and Return activates/opens it
- [ ] `clip` opens detail; search returns rows; Return copies
- [ ] Bare `n` opens Notes detail or shows onboarding "Choose a Notes root folder"
- [ ] Settings opens from menu bar and saves a non-destructive setting that survives restart
- [ ] Doctor reachable on a **default** install through menu bar or Settings recovery entry outside Commands gating; optional `cmd doctor` mirror may also work
- [ ] Export diagnostics through the recovery entry (or optional `cmd export-diagnostics` mirror) writes `~/Library/Logs/Luma/diagnostics.json` with populated fields
- [ ] No new `~/Library/Logs/DiagnosticReports/Luma-*.ips` during the smoke run
- [ ] `~/Library/Application Support/Luma/crash-log.txt` path is known and breadcrumbs are appended on hotkey/action failure
- [ ] Permission denial (AX / EventKit / Automation) shows a row/banner/status, not silent empty

## MVP Non-Goals

The short-term MVP does **not** require:

- Full Media/Records capture, CSV export, category/status filters
- Full Wordbook review/import/export
- Secrets vault unlock/copy/auto-clear flows on the default path
- Window layouts presets on the default path
- Menu bar item search on the default path
- Browser tabs / Automation on the default path
- Kill process / force-kill on the default path
- Windows deferred module registration
- Complex Workbench/Capture/CurrentProject flows
- Full custom user-script platform (Commands user scripts)
- Perfect docs polish beyond MVP-critical mismatches (PERMISSIONS stale, Windows manifest, crash-log path, diagnostics fields, hotkey save no-op)

## Risk Register

Risks that still affect MVP. "MVP Action Needed?" = whether it must be handled inside MVP scope (not how to implement).

| Risk | Related Symptoms | Contracts | Impact | Evidence | MVP Action Needed? |
| --- | --- | --- | --- | --- | --- |
| AppKit/MainActor warnings + `.ips` executor crashes | S-025, S-017, S-014 | C-APPKIT-001/002/003, C-ASYNC-001, C-TEST-003 | App crashes; no stable process | Compiler warnings; three `.ips` with `@objc` AppKit override frames | **Yes** — MVP cannot ship without a staying-running app |
| Hotkey p95 ≈ 8.3s | S-002, S-001 | C-HOT-002, C-ASYNC-003 | Launcher feels unusable | `latency-report.json` | **Yes** — must bring under short-term 1s ceiling; contract 50/80ms is the long-term target |
| Diagnostics gated by Commands default-off | S-020, S-021 | C-DIAG-001, C-DEFAULT-005 | No recovery/support evidence on default install | Missing `diagnostics.json`; Phase 5 facts | **Yes** — recovery must be reachable on default install |
| Tests vs production divergence | S-026 | C-TEST-001/004, C-REVIEW-001 | Green tests mask real failures | Harness ≠ `AppCoordinator.start()`; signed-app paths uncovered | **Yes** — MVP acceptance must include signed-app manual QA, not only SwiftPM |
| Config corruption visibility incomplete | S-024 | C-FAIL-006, C-PERSIST-002, C-DIAG-002 | Doctor cannot see clipboard quarantine or read-failure fallback | Memory-only registry; split quarantine; silent read fallback; empty payload fields | **Yes (MVP-critical subset)** — doctor must at least surface `JSONConfigPersistence` quarantine and not show empty diagnostics |
| Large clipboard history | S-016 | C-PERSIST-001, C-CACHE-002, C-HOT-001 | Slow detail open; corrupt backup; privacy risk | 38MB `clipboard-history.json` + `.corrupt-*.bak` | **Yes (MVP-critical subset)** — must not block `clip` detail open or paste-directly; full history cleanup can be post-MVP |
| Default/docs mismatch | S-019, S-020, S-024 | C-DEFAULT-004, C-DIAG-004 | Users misled about defaults, hotkey edit, crash-log path | PERMISSIONS stale; Windows manifest; `HotkeyConfig.save()` no-op; crash-log path | **Yes (MVP-critical subset)** — fix docs/manifest honesty, not full docs polish |
| Menu bar Show bypass | S-003 | C-UI-001, C-ASYNC-002 | Inconsistent visibility on rapid sequencing | Code fact (PRODUCT_FLOWS) | **Yes (verify-only)** — verify no stuck state; full alignment with Carbon guards can be post-MVP |
| Permission banner timing | S-015 | C-FAIL-001, D-010 | Hidden failure or noisy prompts | Lazy AX policy; partial row enumeration | **Yes (verify-only)** — verify denial shows actionable row on MVP surfaces (Apps, Clipboard paste, Notes, Todo if kept) |
| Restart state loss | S-023 | C-PERSIST-001, C-CACHE-001 | Lost config/session | JSON read-failure silent fallback | **Yes (MVP-critical subset)** — enabled modules + Notes root + Settings must persist; full session resume can be post-MVP |

## Open Decisions

Product trade-offs that need a human decision (not resolvable from phase facts alone):

1. **Todo MVP default-on vs deferred?** Stay default-on (D-012) and verify EventKit denial is actionable, or defer because EventKit is the only high-sensitivity permission on the default path (MVP Principle 2)?
2. **Translate Core P1 candidate vs parked?** Phase 8.5 calibration: Translate is a Core P1 candidate, not a P0 gate. Confirm only if its P1 work should be parked entirely.
3. **Quicklinks/Snippets Core P1 candidates?** Phase 8.5 calibration: both are Core P1 candidates, not P0 gates and not dependencies for Phase 9 P0 slices.
4. **Projects: fully deferred, or retain minimal Settings root/path capability?** Recommendation: park complex Workbench/Capture; retain only the `CurrentProjectService` minimal path needed for Snippets/Quicklinks template variables. Confirm.
5. **Diagnostics reachability resolved in Phase 8.5.** Recovery entry must be reachable outside Commands default-off gating. Menu bar or Settings recovery entry is required; Commands built-ins may remain mirrored command entries but cannot be the only path.
6. **Short-term hotkey latency acceptance: 1s emergency ceiling, or insist on 80ms contract ceiling?** Recommendation: 1s emergency floor for MVP validation so S-002 does not block all MVP work; 50/80ms remains the documented contract target. Confirm.
7. **Commands module default resolved for P0 gating.** User scripts stay parked/default-off. Recovery built-ins must be reachable outside Commands gating; command entries are optional mirrors, not the sole recovery path.
8. **Clipboard history: trim/quarantine as MVP-critical, or only ensure detail open + copy works?** Recommendation: MVP-critical = detail open + copy + paste-directly; full 38MB cleanup post-MVP. Confirm.

## Non-Goals

This phase does **not**:

- Modify Swift source, tests, scripts, manifests, or `ConfigurationStore`.
- Change any module's default on/off switch.
- Delete or merge modules.
- Write Phase 7 refactor steps, implementation plans, or per-file change lists.
- Replace `CONTRACTS.md` or `USABILITY_TRIAGE.md`.
- Run tests, builds, or signed-app launches.
- Stage, commit, or push anything.
