# Roadmap

Active route: **Route C** (Command-First Unified List) per `docs/adr/023-command-first-unified-list.md`.

Implementation reference:

- `docs/adr/023-command-first-unified-list.md`
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`

Historical route records remain in ADRs, but current work should follow Route C and the current codebase rather than old strategy drafts.

## Milestone Status

| Version | Focus | Status |
| --- | --- | --- |
| 0.1 Daily self-use | Command-first launcher: Route C unified list, Open Apps, App Search, Projects, Window Layouts, Translate, Clipboard, Notes, Todo, Wordbook, Snippets, Secrets, Records, Commands, same-panel details | **Complete** |
| 0.2 Stability and trust | Permission UX, first-run guidance, denial/recovery, module diagnostics, warmup/teardown correctness | **In progress (current)** |
| 0.3 UX refinement | Better ranking, cleaner empty states, stronger keyboard-only flows, visual consistency, detail navigation | **In progress (current)** |
| 0.5 Distribution readiness | Signed distribution, regression checklist, launch-on-login polish, repeatable release process | Planned |

## Current Product Priorities

- Close remaining permission-denied and recovery paths so they feel intentional, not incidental.
- Tighten keyboard-only confidence across search, action panels, and detail navigation.
- Keep result ranking intuitive for app names, commands, aliases, pinyin, and exact triggers.
- Maintain visual consistency without drifting back into a dashboard product shape.
- No new feature scope until 0.2/0.3 work is clean.

## Current Module Scope

- Todo: EventKit pass-through. Trigger `t`. No Luma-owned task database.
- Wordbook: same-panel review and manage flows. See `docs/adr/009-todo-wordbook-v01.md`, `docs/adr/013-wordbook-back-in-panel.md`, `docs/adr/016-wordbook-daily-plan.md`, `docs/adr/018-wordbook-three-button-grade.md`.
- Snippets and Secrets: active built-ins from ADR-010; keep secrets explicit, locked, and out of general search.
- Records: local-only logbook from ADR-011. No metadata fetch or social features in v1.
- Notes: Markdown file index and Typora launcher from ADR-008. Notes Graph remains out of scope.

## Validation Priorities

- `swift test` remains green.
- Scripted smoke remains current via `scripts/qa/run_full_smoke.sh`.
- Manual checklist stays aligned with Route C in `docs/MANUAL_QA_CHECKLIST.md`.
- Recorded product-review passes should capture functional, UX, and visual findings separately.

## Historical References

- ADR-006 and ADR-007 remain as historical route records only.
