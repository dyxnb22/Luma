# P3.3 Performance Budgets Report

**Date:** 2026-07-07  
**Phase:** 19 — P3.3  
**Scope:** Documentation + `export_latency_report.sh` threshold modes only.

---

## 1. Files changed

| File | Change |
|------|--------|
| `docs/ENGINEERING.md` | Release-gating vs aspirational vs test-only performance tables |
| `docs/QA.md` | § Performance Gate — collection paths, RC ceilings, `LUMA_QA=1` prerequisite |
| `scripts/qa/export_latency_report.sh` | `LUMA_RELEASE_GATE=1` mode (1000/60 ms); combined p95 informational |
| `P3_PERFORMANCE_BUDGETS_REPORT.md` | This report |

---

## 2. Before / after

| Before | After |
|--------|-------|
| Single ENGINEERING table: 5 metrics all with hard ceilings | **3 RC blockers** + aspirational table + test-only table |
| Hotkey 50/80 ms implied release-blocking | RC hotkey **1000 ms** emergency; 50/80 ms aspirational |
| `export_latency_report.sh` default 50/30 only | `LUMA_RELEASE_GATE=1` → 1000/60 for RC |
| Unclear if smoke runner produces `latency-report.json` | Documented: smokes **do not** set `LUMA_QA=1` |

---

## 3. Release-gating budgets

| Budget | RC ceiling | Artifact | Field |
|--------|------------|----------|-------|
| Hotkey p95 | 1000 ms | `latency-report.json` | `hotkeyP95Milliseconds` |
| Keystroke p95 | 60 ms | `latency-report.json` | `keystrokeP95Milliseconds` |
| Diagnostics export | Payload complete | `diagnostics.json` | See `docs/QA.md` § P0 gate |

**Not gated:** `combinedP95Milliseconds`, module `handle`, panel hide, `LauncherPerfCounters`.

---

## 4. Collection method

1. **Diagnostics:** `./scripts/run_p0_smokes.sh` (`LUMA_QA_EXPORT=1` internally via runner).
2. **Latency report:** Separate session — `LUMA_QA=1 build/Luma.app/Contents/MacOS/Luma`, exercise hotkey + keystrokes, quit → `~/Library/Logs/Luma/latency-report.json`.
3. **Validate:** `LUMA_RELEASE_GATE=1 ./scripts/qa/export_latency_report.sh`.

---

## 5. Known gap

- No automated hotkey/keystroke driver in smoke runner (by design — avoids `LUMA_QA=1` side effects on smokes).
- `diagnostics.json` `latencyP95Milliseconds` is combined samples — use `latency-report.json` for per-metric RC checks.

---

## 6. Build

```bash
swift build   # ✅ (docs/script only)
```

---

## 7. P3.3 verdict

**Go** — ENGINEERING/QA aligned with instrumentation; RC vs aspirational split explicit.

**Next:** Phase 20 release hardening (`docs/QA.md` § Release Candidate Gate).
