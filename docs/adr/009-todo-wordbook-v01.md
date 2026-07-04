# ADR-009: Todo + Wordbook in v0.1

## Status

Accepted. Partially superseded by ADR-013 (Wordbook review panel → in-panel detail).

> **Historical / amended by ADR-023 and ADR-032.** Do not implement dashboard/card or home-row instructions in this ADR. Route C uses prefix triggers and same-panel detail only.

Date: 2026-06-22

## Context

Two daily-driver use cases have been keeping heavy apps RAM-resident on the user's machine:

1. **TODO capture lives in Notion.** Notion stays open (500 MB - 1.5 GB Electron) primarily so the user can drop a task into an inbox. Closing Notion is blocked by "where do I put the next thing I want to remember".
2. **Vocabulary review lives in TechWordPet.app.** A separate native AppKit pet window with its own SQLite at `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`. Functionally useful, but it is a second tray icon and a second data ownership boundary for the same user.

The previous v0.1 row in `docs/ROADMAP.md` explicitly listed `Do Not Do: Wordbook`. That decision was correct given a "ship the minimum launcher" framing. It is wrong under the current framing of "absorb the workflows that keep heavy apps open so we can close them".

`NON_GOALS.md` also has `Wordbook inside the launcher v1 path`. This ADR explicitly carves out the Wordbook take-over described below and supersedes that line.

## Decision

### Todo

Todo is **a capture and triage front-end for system Reminders (EventKit). It does not own any TODO data.**

Concretely:

- Trigger keyword: `t ` (also accept `todo `).
- Capture: `t buy milk` creates an EKReminder on the default list.
- Optional structured time suffix on capture: `today 15:00`, `tomorrow 9:30`, `+30m`, `+2h`. Natural-language parsing is **out of scope for v0.1**.
- Empty query under the trigger lists today's due reminders (max 8 rows), ordered by due time then creation time.
- Primary action on a listed reminder: mark complete. Reminder bodies, lists, priorities, sub-tasks, attachments, recurrence -> all delegated to Reminders.app.
- Permission: request `EKEntityType.reminder` access lazily on first use; deny gracefully (show one-row "Grant Reminders Access" result that opens System Settings).
- Dashboard card: 1 slot. Card primary action focuses the launcher with `t ` pre-filled.

Explicit Non-Goals for Todo:

- Projects, tags, sub-tasks, attachments (live in Reminders.app).
- A second Luma-owned TODO database.
- Natural-language date parsing in v0.1.
- Recurrence editing in Luma (Reminders.app handles it; we just read what is set).
- Cross-list management UI (Reminders.app handles it).

### Wordbook

Wordbook is **the full successor to TechWordPet.app.** After v0.1 ships, the user can delete TechWordPet.app.

Concretely:

- On first launch, copy `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3` to `~/Library/Application Support/Luma/Wordbook/wordpet.sqlite3`. Do not overwrite if the Luma-owned copy already exists. After migration the Luma copy is the **only** source of truth.
- `WordbookStore` switches to READWRITE mode against the Luma-owned DB. Schema is preserved as-is from wordbot (no rename, no migration of columns).
- Search trigger: `word ` keyword + free text matches `term / meaning / example / category` (LIKE, ordered by `wrong_count DESC, review_count ASC`).
- Card primary action: open the **Wordbook Review Panel**.
- 9-stage Ebbinghaus schedule preserved unchanged (already implemented in `ReviewScheduler.swift`).
- Review records: `recordReview(wordID, familiarity)` updates `review_stage`, `next_review_at`, `review_count`, `wrong_count`, `last_review_at`. `mastered_at` set when stage reaches 9 with `familiarity == .known`.

### Wordbook Review Panel (dedicated window)

Review is a sit-down workflow (3-15 minutes per session). It does **not** belong inside the 860x540 launcher panel. Holding the launcher panel open for the duration of a review breaks the Route B hot-path discipline (`docs/specs/PERFORMANCE.md`: panel hide p95 ≤ 20 ms; users expect Esc to close).

Therefore:

- Review opens a separate `NSPanel`, owned by Luma, sized ~480 x 320 pt.
- Style: liquid-glass stack consistent with the launcher (visual effect view + 1pt inner border + corner radius), but operationally **independent**: closing it does not affect launcher state.
- State machine: `Question` (term + phonetic + 3 buttons Known/Fuzzy/Unknown) -> `Revealed` (meaning + example + Speak + Next) -> next word. Empty queue closes the panel with a "Done for today" final view.
- Speech via `NSSpeechSynthesizer` (or `AVSpeechSynthesizer`) with British English voice, matching the TechWordPet `settings.voice_accent=uk` preference.
- Esc closes the panel; Cmd+W closes; clicking outside does not close (review is intentional focus).

### Explicit Non-Goals for Wordbook v0.1

- Floating desktop pet mode (the W bubble that follows you around). If we want it later we add a separate `NSPanel`, but it is **not** the v0.1 entry point. Launcher card click + `word ` trigger are the v0.1 entry points.
- CSV / Markdown import UI. Wordbot keeps the existing 1341-word seed. New imports go through ad-hoc tooling for now.
- Daily goal editing in Luma. The `settings` table value is read; editing happens via SQL or Reminders.app for now.
- ChatGPT-paste parser.
- Cross-Spaces / full-screen pet overlay behavior.
- A separate Web/Docker version (was wordbot's V1 alternative — superseded by this ADR).

## Consequences

Positive:

- Closing Notion stops costing the user their TODO inbox. Closing TechWordPet.app stops costing the user vocabulary review. Both are direct hits on the user's stated goal of reducing resident memory.
- Todo as an EventKit pass-through has near-zero ongoing maintenance: Apple owns the data store, backup, and cross-device sync.
- Wordbook reuses existing `ReviewScheduler` (9-stage already implemented) and existing SQLite schema (no migration).
- Dashboard card count after this change: Translate, Clipboard, Notes, Todo, Wordbook = 5. Still ≤ 8 ceiling from ADR-007. Two slots remain for the next round.

Negative:

- Two new modules ship in v0.1 instead of one. Risk: scope creep in the 0.1 window.
- Wordbook review panel is a new window class outside the launcher panel; introduces a new code path for panel show/hide and visual style maintenance.
- EventKit permission prompt is a first-launch friction. Mitigation: lazy prompt on first `t ` capture, not on app start.
- After migration, the wordbot directory becomes read-only reference. The user must not edit `~/wordbot/data/wordpet.sqlite3` and expect Luma to see it. The Luma-owned path becomes authoritative.

## Implementation Notes

Phases (rough sequencing, see TaskList for canonical breakdown):

1. EventKit `RemindersService` + Info.plist usage description.
2. Rewrite `TodoModule` with structured time parser + capture / list / complete actions.
3. Wire Todo into `BuiltInModules.makeAll()` + `FeatureCatalog.dashboardCoreCards()`.
4. `WordbookStore` migration step + READWRITE upgrade + `recordReview` method.
5. `SpeechService` (British English).
6. `WordbookReviewPanel` standalone NSPanel.
7. Wire Wordbook into `BuiltInModules.makeAll()` + dashboard card.
8. Update `docs/ROADMAP.md` (replace `Do Not Do: Wordbook` with new row), `docs/NON_GOALS.md` (remove the Wordbook v1 line, keep the more specific carve-outs above), `docs/specs/PERFORMANCE.md` (add Todo `handle` budget and confirm Wordbook `handle` budget).

Files touched in `~/wordbot`: **none**. The migration is a one-way copy. The wordbot directory remains untouched as a backup; the user can delete it manually after confirming Luma's copy works.

Reminders.app default-list name is the user's existing default (whatever EventKit returns from `EKEventStore.defaultCalendarForNewReminders()`). No Luma-specific list is created.
