# ADR-018: Wordbook Three-Button Grade

## Status

Accepted (2026-06-22)

## Context

TechWordPet used Known / Fuzzy / Unknown. Users want clearer semantics: progress up, reset, or mark as already learned (never due again).

## Decision

| UI | `WordFamiliarity` | Effect |
| --- | --- | --- |
| 不认识 | `.unknown` | Stage → 0, wrong_count++ |
| 认识 | `.known` | Stage +1 per Ebbinghaus schedule |
| 已学过 | `.mastered` | `mastered_at` set, `next_review_at` ~100 years ahead, skip reveal |

- `.fuzzy` retained in enum/DB; scheduler treats it as `.known` for backward compatibility.
- Shortcuts: `1` / `2` / `3` left-to-right matching buttons.
- Mastered grades call `advance()` immediately (no answer reveal).

## Consequences

- Users cannot mark "fuzzy" in new sessions; legacy rows unchanged.
- `progressSnapshot.mastered` increments when words are marked mastered.
