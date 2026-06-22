# ADR-011: Media Tracker (trigger-only)

## Status

Accepted.

Date: 2026-06-22

## Context

The user logs movies, TV, anime, games, and books in Notion/Douban/Letterboxd tabs. Capture is bursty (after finishing media); read-back is weekly. A launcher-native log replaces those tabs without building a social or discovery product.

## Decision

### Media module

- Five categories: movie, tv, anime, game, book. Flat list, one JSON store at `~/Library/Application Support/Luma/Media/media.json`.
- Trigger: `m ` (also `media `). **No dashboard card** — trigger-only per Route B 8-card ceiling discipline.
- Capture DSL: `m <title> [category] [rating] [status]` with one-line greedy parse.
- Search: fuzzy title match; empty `m` shows recent items + "Media Log" entry.
- Detail view: category tabs, status filter, sort, CRUD table, CSV export to Downloads.
- Primary search action: open edit sheet. Secondary (Tab): copy `title — rating/10`.

### Explicit non-goals

Douban/Letterboxd clone, discovery, streaming integration, posters, episode-level TV tracking, cloud sync, TMDB/metadata fetch in v1. See `docs/NON_GOALS.md` Media section.

## Consequences

- `ModuleIdentifier.media` added; not in `FeatureCatalog.dashboardCoreCards()`.
- Detail view opened via `m log`, empty `m` manage row, or post-capture edit flow.
- `MediaActions.openDetail` bridges module perform → launcher panel.
