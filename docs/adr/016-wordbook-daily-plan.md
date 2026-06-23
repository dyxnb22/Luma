# ADR-016: Wordbook daily plan and mixed session engine

## Status

Accepted (2026-06-22). Supersedes ADR-009 §Wordbook "daily goal read-only / no editing in Luma".

## Context

Wordbook must become a full in-panel spaced-repetition system (TechWordPet successor) with adaptive new-word quotas and daily progress.

## Decision

1. **Settings table** keys: `daily_stats_date`, `daily_new_seen`, `daily_wrong_count`, `daily_reviewed`, `voice_accent`.
2. **Quota formula** copied from TechWordPet (`dailyNewLimitForDueCount`).
3. **`WordbookSessionPlanner` actor** mixes review/new cards with ratio targets (15/25/35%) and wrong-answer down-weighting.
4. **`WordbookDetailView`** three states: home (progress card), session, done.
5. **`daily_review_log` table** tracks streak and per-day stats.

## Consequences

- Cross-day reset runs on detail open and grade, not mid-session.
- Word manager + CSV import live in `WordbookManageView`.
- Floating pet remains a non-goal.
