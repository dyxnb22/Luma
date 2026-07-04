# QA Artifacts

This folder mixes **current acceptance entry points** with **timestamped run outputs**. Do not treat screenshots or reports here as product spec.

## Current acceptance (use these)

| Entry | Purpose |
| --- | --- |
| [docs/MANUAL_QA_CHECKLIST.md](../docs/MANUAL_QA_CHECKLIST.md) | Canonical manual regression checklist |
| [docs/RECORDED_QA_BRIEF.md](../docs/RECORDED_QA_BRIEF.md) | Recorded walkthrough scope and findings format |
| [docs/qa/LAUNCHER_NAVIGATION_AUDIT.md](../docs/qa/LAUNCHER_NAVIGATION_AUDIT.md) | Open navigation/shortcut gaps (temporary; fold into specs when closed) |
| [docs/qa/AUTOWORKFLOW_UI_ACCEPTANCE.md](../docs/qa/AUTOWORKFLOW_UI_ACCEPTANCE.md) | Auto Workflow UI pass checklist |
| `scripts/qa/run_full_smoke.sh` | Scripted launcher smoke |
| `scripts/run_recorded_review.sh` | One-command build + smoke + recorded review setup |
| [RECORDED_REVIEW_TEMPLATE.md](RECORDED_REVIEW_TEMPLATE.md) | Findings log template for recorded sessions |

Engineering context: [docs/ENGINEERING_PACKAGE.md](../docs/ENGINEERING_PACKAGE.md).

## Historical run outputs (reference only)

Timestamped folders and round snapshots from past QA sessions. Useful for comparing UI regressions; **not** authoritative behavior.

| Path | Contents |
| --- | --- |
| `autoworkflow-ui-20260701-*` | Auto Workflow UI evidence (screenshots, `cc-loop` logs, `report.md` where present) |
| `final-ui-acceptance-20260701-122711/` | Full UI acceptance round (screenshots + `report.md`) |
| `final/` | Prior final regression screenshots and `findings.md` |
| `round-3/` | Round 3 screenshots and cross-module captures |
| [SUMMARY.md](SUMMARY.md) | Aggregated round counts from earlier sessions (may be stale) |

When behavior conflicts with code or frozen specs, trust **code + `docs/specs/`**, not old screenshots.
