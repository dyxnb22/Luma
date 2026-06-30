# Todo

## Goal

Create and check off reminders from the launcher without switching to Reminders.app. Luma is a pass-through — Apple Reminders is the source of truth.

## Triggers

- `t` / `todo` — list today's due reminders
- `t <task>` — natural-language capture; Return creates the reminder
- `t <task> tomorrow 9:00` — capture with parsed due date

## Natural Language Parsing

`TodoTimeParser` extracts a due date from the task text:

| Input pattern | Result |
|---------------|--------|
| `tomorrow` | next calendar day, 09:00 |
| `tomorrow 9:00` | next calendar day at 09:00 |
| `Jan 5` / `5/1` | absolute date |
| no time words | no due date (Inbox) |

Unrecognized patterns create a reminder with the full text as title and no due date.

## Data Model

No Luma-owned persistence. All state lives in Apple Reminders (EventKit). Luma caches today's due list in memory with a short TTL to avoid per-keystroke EventKit IPC.

## Actions

- **Add Reminder** (Return on capture row) — creates via EventKit
- **Mark Complete** (Return on due row) — marks the reminder complete
- **Uncomplete** — available from detail view
- **Allow Reminders Access** — shown when permission is not yet granted

## Permissions

- **Reminders** (EventKit) required for all functionality.
- If not determined: permission row shown when query is empty.
- If denied: permission row with link to System Settings.
- No silent failure — the permission state is always surfaced.

## Detail View

`TodoDetailView` shows four tabs: Today / Inbox / Upcoming / Completed. Counts are shown on tab labels when non-zero.

## Warmup

`warmup` requests authorization (if already granted) and pre-fetches today's due list. In-memory cache TTL is defined in `CacheTTL.dueListSeconds`. Phase 1 (no disk I/O).

`TodoChangeHub` broadcasts cache invalidation when reminders are created, completed, or updated, keeping open detail views live.

## Implementation Entry

- Module: `Sources/LumaModules/Todo/TodoModule.swift`
- Time parser: `Sources/LumaModules/Todo/TodoTimeParser.swift`
- Detail view: `Sources/LumaApp/Launcher/TodoDetailView.swift`

## Non-Goals

- No Luma-owned task database (see ADR-009).
- No recurring tasks, subtasks, or priority levels in v1.
- No Calendar integration (Reminders only).
