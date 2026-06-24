# ADR-020: Wordbook v1 Frozen Scope

## Status

Accepted (2026-06-24)

## Context

Wordbook is feature-complete for daily self-use: single SQLite wordbook, 9-stage Ebbinghaus scheduling, adaptive daily new-word quota, three-button grading, and in-panel home/session/done/manage flows (ADR-009 → ADR-018). Remaining work was product-definition polish, not greenfield implementation.

## Decision

### Product definition

Wordbook is **one local wordbook** inside Luma. Users import or add words, open `word` in the launcher, and grade cards with **不认识 / 认识 / 已学过** (keys `1` / `2` / `3`). Space reveals meaning. Scheduling and daily new-word limits are automatic.

**Today is done** when either:

1. Due queue is empty **and** daily new-word quota is exhausted (or no new words remain), or
2. The user completes ≥ 30 cards in one session and chooses **Done for today** in the soft-cap prompt (`daily_target_acked` prevents repeat prompts that day).

### Frozen pillars

| Area | Frozen choice |
| --- | --- |
| Books | Exactly **one** book; no `book_id`, no multi-deck |
| Storage | `~/Library/Application Support/Luma/Wordbook/wordpet.sqlite3` |
| Schema | `words` + `settings` + `daily_review_log` + `words_fts` (no new columns without ADR) |
| Trigger | Launcher keyword **`word`** |
| Grading | Three buttons only; `.fuzzy` is **legacy read-only** (scheduler maps to `.known`; UI never writes it) |
| Scheduling | 9 intervals (5m … 30d) + adaptive new-word quota formula |
| UI | In-panel sub-states: `home`, `session`, `done`, `manage` |
| Network | **Zero cloud** except opt-in **Suggest IPA · online** (`dictionaryapi.dev`, 5s timeout) |
| Category | Free-form tag for filtering only; no category management tools |

### Settings keys (frozen set)

- `daily_stats_date`, `daily_new_seen`, `daily_wrong_count`, `daily_reviewed`
- `daily_mastered` (display only; excluded from accuracy numerator/denominator)
- `daily_target_acked` (soft-cap acknowledgment date)
- `voice_accent`

### Accuracy

`accuracyToday = (daily_reviewed - daily_wrong_count) / daily_reviewed`. **已学过** (`.mastered`) increments `daily_mastered`, not `daily_reviewed`.

### Must ship for v1 freeze

- Wrong Words button on home
- Empty-state guidance + migration notice
- Export CSV (symmetric with import)
- Soft session cap (30 cards) + `daily_target_acked`
- Same-day session resume (do not `startNewSession` when cutoff is today and cards were shown)
- IPA button labeled **Suggest IPA · online**

## Non-goals (require new ADR to revisit)

1. Multi-book / multi-deck / multi-language switching
2. Custom SRS intervals or FSRS/SM-2
3. AI-generated definitions, examples, or translations
4. Online dictionary beyond opt-in IPA
5. Dictation, spelling drills, or alternate card types
6. Custom card templates
7. Learning curves, heatmaps, long-range analytics
8. Review push notifications or menu-bar badges
9. Floating desktop word pet
10. Cloud sync, accounts, cross-device
11. Category rename/merge/delete tools
12. Separate label/tag system beyond `category`
13. PDF/EPUB import (CSV/TSV only)

## Consequences

- Wordbook PRs labeled `feature` must link a new ADR if they touch frozen pillars above.
- Bugfix PRs may adjust layout, copy, edge cases, performance, and tests without ADR.
- Legacy `~/wordbot` migration remains **one-way** (ADR-009).

## References

- ADR-009, ADR-013, ADR-016, ADR-018
- `docs/NON_GOALS.md` § Wordbook
