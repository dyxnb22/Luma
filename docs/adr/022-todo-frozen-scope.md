# ADR-022: Todo v1 Frozen Scope

## Status

Accepted (2026-06-24)

> **Historical / amended by ADR-023 and ADR-032.** Do not implement dashboard/card instructions in this ADR. Todo entry is `t` / `todo` trigger and detail panel only.

## Context

Todo is an EventKit pass-through for system Reminders (ADR-009). The module already supports launcher capture (`t `), structured time suffixes, today's due list with cache, and a detail panel. Remaining work is **freezing** the product to “daily capture + triage desk” — not building a Todoist/Things replacement.

References: Todoist Quick Add / filters (capture + light views); Things Today / Upcoming / Anytime (date buckets); Microsoft To Do My Day / Planned / Completed (smart lists). Luma takes only the skeleton: quick capture, date views, a few smart lists.

## Decision

### Product definition

Luma Todo is **Reminders quick capture + today's processing desk**. Users drop tasks from the launcher or detail panel; Reminders.app remains source of truth for sync, lists, recurrence, and rich editing.

### Frozen pillars

| Area | Frozen choice |
| --- | --- |
| Data | **EventKit / Reminders only** — no Luma-owned TODO database |
| Trigger | `t ` / `todo `; empty `t` lists today due (cached) |
| Detail tabs | **Today** (due today + overdue) · **Inbox** (no due date) · **Upcoming** (tomorrow+) · **Done** (completed last 7 days) |
| Capture | Structured suffixes only (`+30m`, `today 15:00`, `明天 9点`, `周五 15:00`, `下午3点`, etc.) — no NLP library |
| Actions | Create, complete, **uncomplete**, edit title/date, **clear date**, schedule Today/Tomorrow, open Reminders.app |
| Launcher cache | `handle` reads today due from TTL cache; detail refresh may hit EventKit (cancel prior task) |
| Dashboard card | Subtitle **Today + quick capture**; primary action focuses launcher with `t ` |

### Explicit non-goals

- Projects, tags, sub-tasks, attachments, collaboration
- Cross-list management UI
- Recurrence editing in Luma
- Natural-language date parsing (beyond fixed structured rules)
- A second TODO database or export format owned by Luma

### Quality bar

- Unit tests: `TodoTimeParser`, tab/list kinds, `clearDueDate` / `uncomplete` action encoding
- Acceptance: `swift test --filter TodoTimeParserTests` plus related module tests

## Consequences

Positive:

- Clear scope boundary; low maintenance (Apple owns persistence and sync)
- Four-tab IA matches proven “capture + smart lists” pattern without feature creep

Negative:

- Advanced Reminders workflows still require Reminders.app
- Inbox / completed sorting depends on EventKit metadata (`creationDate`, `completionDate`)

## Supersedes

Extends ADR-009 Todo section: v0.1 non-goals on natural language are relaxed to **structured Chinese/English rules only**; detail panel grows from three tabs to four (adds Inbox) and adds undo / clear date.
