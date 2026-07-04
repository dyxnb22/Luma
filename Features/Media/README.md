# Media (Records)

## Goal

Personal logbook for movies, books, games, TV series, and albums. Quick-capture ratings from the launcher; review and filter from the detail view. Local-only, no metadata fetch, no social features.

User-facing name is **Records**. Technical module identifier is `luma.media`.

## Default State

**Default-off.** Enable in Settings → Modules.

## Triggers

- `m` / `media` / `rec` / `record` / `log` — open Records detail
- `m <title>` — search existing entries
- `m <title> movie 9` — capture: title + category + rating, Return logs it
- `m log` — open the full log detail view directly

## Capture Syntax

`MediaParser` extracts category and rating from free-form text:

| Category keywords | Parsed as |
|-------------------|-----------|
| `movie` / `film` | Movie |
| `book` | Book |
| `game` | Game |
| `tv` / `show` / `series` | TV |
| `album` / `music` | Album |

A trailing integer (1–10) is captured as rating. Everything else is the title.

Examples:
- `m Oppenheimer movie 9` → Movie · Oppenheimer · 9/10
- `m Dune book` → Book · Dune · no rating
- `m hades game 10` → Game · Hades · 10/10

## Data Model

Stored in `~/Library/Application Support/Luma/media.json`.

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | |
| `title` | String | searchable |
| `category` | MediaCategory | movie / book / game / tv / album |
| `status` | MediaStatus | want / in-progress / completed / dropped |
| `rating` | Int? | 1–10 |
| `notes` | String? | plain text only |
| `completedAt` | Date? | |
| `createdAt` | Date | |

## Actions

- **Log** (Return on capture row) — creates entry
- **Search** (Return on result row) — opens entry in detail
- **Edit / Delete** — in detail view; persists immediately
- **Export CSV** — writes to Downloads folder, opens in Finder
- **Tab on result** — copies `title — rating/10` to clipboard

## Warmup

`warmup` loads all entries from `media.json` into `cachedItems: [MediaItem]`. Subsequent `handle` calls are memory-only. Warmup tier is `onDemand`; do not add Media to `BuiltInModules.fastModuleIDs`.

## Implementation Entry

- Module: `Sources/LumaModules/Media/MediaModule.swift`
- Store: `Sources/LumaModules/Media/MediaStore.swift`
- Parser: `Sources/LumaModules/Media/MediaParser.swift`
- Index / search: `Sources/LumaModules/Media/MediaIndex.swift`
- Detail view: `Sources/LumaApp/Launcher/MediaDetailView.swift`

## Non-Goals

See `docs/NON_GOALS.md` — Media section. No metadata enrichment (TMDB/OMDb/Google Books), no streaming integration, no social/discovery features, no episode-level TV tracking, no poster/cover-art fetching in v1.
