# P2 Decision Matrix (Phase 14.2)

**Date:** 2026-07-07  
**Baseline:** `bc966c29`  
**Companion:** `P2_SCOPE_AUDIT.md`, `P2_ROADMAP.md`

Legend: **Do Now** = first executable P2 slices | **Defer** = planned later P2/P3 | **Drop** = explicitly not doing | **Needs User Decision** = product call required

---

## 1. `LauncherSessionState`

| Field | Value |
|-------|-------|
| **Decision** | **Defer** (keep test-only through P2); **Drop** full production wiring in P2 |
| **Why not now** | 7/11 events unwired; wiring duplicates `visibilitySession` + coordinator; `panelHideCompleted` gap leaves shadow stuck |
| **MVP impact** | None if frozen; risk if half-wired |
| **Refactor risk** | **High** if promoted to SoT |
| **Minimal slice** | P2.1: document "test-only spec + 4 legacy effect hooks" in `LAUNCHER_STATE_AUDIT.md` §6; P2.5+: optional delete after inlining `cancelAllTasks` / `clearDetailModeState` calls |
| **Acceptance** | No new `applySessionEvent` call sites in P2; illegal-transition tests still pass |
| **P3 option** | Delete reducer or promote `visibilitySession` only — not both |

---

## 2. `docs/PERMISSIONS.md` stale defaults

| Field | Value |
|-------|-------|
| **Decision** | **Do Now** (P2.1) |
| **Why now** | Pure doc fix; closes C-DEFAULT-004; zero runtime risk |
| **MVP impact** | None |
| **Refactor risk** | **None** |
| **Minimal slice** | Correct Default column to match manifests + `docs/MODULES.md` D-012; fix "Menu Items" naming |
| **Acceptance** | Table matches `ModuleWarmupDefaults` for every module; manual diff vs `MODULE_MATRIX.md` |

---

## 3. Windows deferred / manifest mismatch

| Field | Value |
|-------|-------|
| **Decision** | **Do Now** (P2.1) |
| **Why now** | One-line manifest metadata; module stays unregistered |
| **MVP impact** | None |
| **Refactor risk** | **None** if only `defaultEnabled: false` + comment |
| **Minimal slice** | `WindowsModule.swift` manifest flag; `docs/PERMISSIONS.md` row already says "deferred" |
| **Acceptance** | `BuiltInModules.makeDeferred()` unchanged; not in `ModuleRegistry.allBundles` |
| **Drop** | Fixing `handle()` CGWindow violation while deferred |

---

## 4. Module lifecycle contract (warmup/handle/perform/teardown)

| Field | Value |
|-------|-------|
| **Decision** | **Defer** → **P2.3** |
| **Why not now** | Needs per-module test inventory before code changes |
| **MVP impact** | Medium — proves P0 modules memory-only |
| **Refactor risk** | **Medium** if becomes ModuleHost rewrite |
| **Minimal slice** | Extend `ModuleHandleContractTests` for Apps, Clipboard, Notes; document exceptions table |
| **Acceptance** | Each P0 module has warmup/handle/perform/teardown test or documented exception |
| **Drop** | KillProcess teardown, Wordbook perform redesign, Windows handle fix |

---

## 5. Diagnostic row / status consistency

| Field | Value |
|-------|-------|
| **Decision** | **Defer** → **P2.2** |
| **Why not now** | Depends on lifecycle clarity; touches `QueryDispatcher` synthesis path |
| **MVP impact** | Medium — UX clarity, not crashes |
| **Refactor risk** | **Medium** — shared infra affects parked modules |
| **Minimal slice** | Document failure taxonomy table; align Apps/Clipboard/Notes cold/permission/empty rows |
| **Acceptance** | Table in `docs/MODULES.md`; tests for disabled→diagnostic, cold→warming, Notes onboarding |
| **Defer parked** | Wordbook/Media/BrowserTabs diagnostic polish |

---

## 6. Default-enabled module slimming (runtime)

| Field | Value |
|-------|-------|
| **Decision** | **Needs User Decision** (Todo default-on); **Drop** runtime flips in P2 without decision |
| **Why** | `MVP_SCOPE.md` Open Decision: Todo EventKit permission surprise |
| **MVP impact** | **High** if Todo turned off without product sign-off |
| **Refactor risk** | Low for doc-only; high for switch changes |
| **Minimal slice** | P2.1 doc alignment only |
| **Acceptance** | Docs match code; no `defaultEnabled` change except Windows metadata |

---

## 7. Deferred / parked module manifest clarity

| Field | Value |
|-------|-------|
| **Decision** | **Do Now** (P2.1) |
| **Why now** | Prevents accidental re-enable; complements PERMISSIONS fix |
| **MVP impact** | None |
| **Refactor risk** | **None** |
| **Minimal slice** | Add `docs/MODULES.md` or `MODULE_MATRIX.md` "Registration status" column: registered / deferred / parked |
| **Acceptance** | Windows, BrowserTabs, etc. explicitly marked deferred; no new registrations |

---

## 8. `handle()` memory-only enforcement

| Field | Value |
|-------|-------|
| **Decision** | **Defer** → **P2.3** |
| **Why not now** | Generic enforcement needs linter or static test, not ModuleHost change |
| **MVP impact** | High for query latency |
| **Refactor risk** | **Low** with test/linter proxy; **high** with host rewrite |
| **Minimal slice** | `scripts/` or test that flags `await` in `handle()` for P0 modules; extend `ModuleHandleContractTests` |
| **Acceptance** | Apps, Clipboard, Notes pass; Windows excluded (deferred) |
| **Drop** | AST rewrite of all modules |

---

## 9. Non-MVP AppKit `@objc` warn cleanup

| Field | Value |
|-------|-------|
| **Decision** | **Defer** → **P2.4** |
| **Why not now** | 78 warns; MVP Clipboard done in P1; bulk fix is churn |
| **MVP impact** | Low for P0 path |
| **Refactor risk** | **Medium** per file (AppKit executor crashes) |
| **Minimal slice** | Snippets, Quicklinks, Translate, Todo detail views — one file per PR |
| **Acceptance** | Scanner warn count drops for touched files; `swift test` + targeted smoke |
| **Defer** | Wordbook, Media, Secrets, Projects, parked views |

---

## 10. Clipboard / Notes UX polish

| Field | Value |
|-------|-------|
| **Decision** | **Defer** (post P2.2 or P3) |
| **Why not now** | Smokes green; polish ≠ governance |
| **MVP impact** | Low |
| **Refactor risk** | Medium — touches MVP detail views |
| **Minimal slice** | Single UX issue per PR (e.g. clipboard search feedback, Notes root CTA) |
| **Acceptance** | Signed smoke + manual QA item |
| **Drop** | Large history refactor in P2 |

---

## 11. `LauncherFlowHarness` vs production wiring

| Field | Value |
|-------|-------|
| **Decision** | **Defer** → **P2.5** (partial align); full parity **P3** |
| **Why not now** | May require `CommandRegistry` + global search config in harness |
| **MVP impact** | Test signal only |
| **Refactor risk** | **Medium** — harness changes can hide real bugs if wrong |
| **Minimal slice** | Label harness tests "logic-only" OR add `configureGlobalSearchModuleIDs` to harness factory |
| **Acceptance** | `launcherFlowHarnessReplaysQuery` uses production-equivalent router config |
| **Drop** | Replacing all integration tests with AppCoordinator E2E in P2 |

---

## 12. Terminable `LUMA_QA_*` smoke runner

| Field | Value |
|-------|-------|
| **Decision** | **Defer** → **P2.5** |
| **Why not now** | Needs script wrapper + `exit(0)` after JSON write in smoke hooks |
| **MVP impact** | Improves CI/manual gate reliability |
| **Refactor risk** | **Low** |
| **Minimal slice** | `scripts/run_p0_smokes.sh` runs each env var, polls `~/Library/Logs/Luma/*-smoke.json`, kills app, exits non-zero on failure |
| **Acceptance** | Single command runs Apps/Clipboard/Notes/Settings; EXPORT optional |
| **Drop** | Rewriting all smoke hooks in one PR |

---

## Decision summary

| Decision | Count | Items |
|----------|-------|-------|
| **Do Now** | 3 | PERMISSIONS defaults, Windows manifest, parked/deferred manifest docs |
| **Defer** | 7 | Session state migration, lifecycle tests, diagnostics, handle enforcement, AppKit warns, harness, QA runner |
| **Drop** | 4 | ModuleHost rewrite, Windows handle fix, full session wiring, runtime default flips |
| **Needs User Decision** | 1 | Todo default-on vs defer |

---

*Phase 14.2 — documentation only.*
