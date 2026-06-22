# Media Tracker Implementation Plan

Status: complete. ADR-011 accepted.
Route: B (Dashboard Widget Single Window) per ADR-007.
Dashboard slot: trigger-only (Option B), confirmed 2026-06-22.
Date: 2026-06-22

## What this module is

A local-only **personal log** of media the user has consumed: movies, TV shows, anime, games, and books. Five categories, one flat list, one JSON store. No social features, no discovery, no posters in v1, no streaming integration.

Think: a fast capture front-end for "I just watched Oppenheimer", plus a clean read-back view for "what did I rate ≥ 8 this year." That's the entire surface.

## What this module is **not** (read first)

If anything below tempts a future feature request, write a new ADR superseding ADR-011. Do not silently grow this module.

- A Douban / Letterboxd / Trakt clone. No social, no comments, no friend feeds, no recommendation engine.
- A discovery tool. We never suggest "what to watch next."
- A streaming integration. No Netflix activity, no Spotify history, no Steam import.
- A poster gallery. No image fetching, no thumbnail caching in v1.
- A review-writing platform. The `notes` field is plain text, ≤ 2000 chars, no markdown rendering.
- Episode-level tracking for TV. One status per series ("in progress", "done"). No "S03E07 watched on..." granularity.
- A time tracker. We don't record how long a game was played.
- Achievements / trophies / completion %.
- Cloud sync, sharing, public profiles.
- AI summarization or auto-generated reviews.
- Per-rewatch records. Rewatching updates `completedAt`, not a separate row.

## Why it fits Luma

- **Capture is a launcher workflow.** `m Oppenheimer movie 9` is a single-line hotkey gesture — exactly what Luma exists for. The 5-second capture replaces opening Douban/Notion/Letterboxd and clicking through a "log new" form.
- **Read-back is rare enough for a detail view.** Users open a media-log app once a week, not every keystroke. A spartan detail surface is fine.
- **Pure local, pure macOS.** No NON_GOALS broken.
- **Reduces RAM pressure**: replaces Notion media databases and browser tabs of Douban / Letterboxd.

## Dashboard slot decision

`docs/adr/007-dashboard-widget-single-window.md` caps Route B at 8 cards. After Snippets + Secrets land, we are at 7 cards.

**Decision: Media is trigger-only. No dashboard card.** Reasons:

- Media capture is bursty (after finishing a movie) and infrequent (a few times a week). It is not "always-on" the way Translate or Clipboard is.
- Read-back lookups go through `m ` search trigger directly, no card needed.
- Keeps a dashboard slot free for future high-frequency entries.

If a future ADR overturns this, adding a card is ~30 lines (FeatureCard + dashboardCoreCards entry + gradient).

## Capture DSL

One line, greedy parse. Inspired by `TodoTimeParser`.

```
m <free-text-title> [category] [rating] [status]
```

Token classifiers (case-insensitive, position-independent):

- **Category**: `movie` / `film`, `tv` / `show` / `series`, `anime`, `game`, `book` / `novel`. Default if unspecified: prompt user via detail view rather than guessing.
- **Rating**: integer 1-10, optionally `9/10` form. Anything outside [1,10] stays in the title.
- **Status**: `planning` / `plan`, `watching` / `reading` / `playing` / `wip`, `done` / `finished`, `abandoned` / `dropped`. Default if unspecified: `done` (capture-time bias: you log after finishing).

Examples:

```
m Oppenheimer movie 9              -> {title: "Oppenheimer", cat: movie, rating: 9, status: done}
m The Three-Body Problem book 10   -> {title: "The Three-Body Problem", cat: book, rating: 10, status: done}
m Cyberpunk 2077 game dropped 4    -> {title: "Cyberpunk 2077", cat: game, rating: 4, status: abandoned}
m Frieren anime watching           -> {title: "Frieren", cat: anime, status: in_progress, rating: nil}
m The Bear                         -> partial — opens edit sheet pre-filled with title only
```

Ambiguity rule: numbers between 1-10 are interpreted as rating, **not** part of the title, unless they appear inside the first token (e.g. `m 1984 book` keeps "1984" as title because it's the leading token).

## Search

`m <query>` with no DSL-recognised tokens triggers search:

- Fuzzy title match across all stored items.
- Top 8 results, ranked by `match_score * 0.6 + recencyBoost * 0.4`.
- Each row: title, category icon, status pill, rating star.
- Primary action: open edit sheet (not "copy" — the user wants to update the record more often than reference it).
- Secondary action `Cmd+Return`: copy `title — rating/10` to clipboard.

`m` with no payload: show last 8 added across all categories (recently-edited bias). Subtitle: "23 items · last added 3 days ago".

Hot path: `handle` reads only the in-memory index. Warmup loads JSON. Edits write through `MediaStore` and refresh the index.

## Detail view

Lives in the Route B same-panel detail container (not a separate window — read-back is brief enough).

Layout:

```
+-------------------------------------------------------------+
| All  Movies  TV  Anime  Games  Books        + Add  Export   |  <- segmented + toolbar
+-------------------------------------------------------------+
| Status: [Done ▾]   Sort: [Recently Completed ▾]             |  <- filter bar
+-------------------------------------------------------------+
| Title                  Cat   Status   Rating  Completed     |  <- NSTableView
| Oppenheimer            🎬    Done     ★9      2026-06-01    |
| The Three-Body Problem 📖    Done     ★10     2026-05-22    |
| ...                                                          |
+-------------------------------------------------------------+
| 23 items · avg rating 7.4 · 12 done this year               |  <- footer stats
+-------------------------------------------------------------+
```

- Category tabs are segmented control; switching filters the table.
- Status filter dropdown: All / Planned / In Progress / Done / Abandoned.
- Sort dropdown: Recently Completed / Recently Added / Rating Desc / Title.
- Click row → edit sheet (same fields as Add).
- Toolbar: + Add (opens sheet with empty form), Export (writes a CSV to ~/Downloads, see below).
- Delete via row context menu or Cmd+Delete keystroke.

Add/Edit sheet fields:

- Title (text)
- Category (popup)
- Status (popup)
- Rating (1-10 slider, optional "no rating" toggle)
- Started date (date picker, optional)
- Completed date (date picker, optional, defaults to today when status flips to Done)
- Notes (multi-line text, ≤ 2000 chars)
- Tags (token field, optional, lowercased on save)

Esc cancels the sheet, Return saves. No confirmation prompt for save — only for delete.

## Data model

```swift
public enum MediaCategory: String, Codable, CaseIterable, Sendable {
    case movie, tv, anime, game, book
}

public enum MediaStatus: String, Codable, CaseIterable, Sendable {
    case planned, inProgress, done, abandoned
}

public struct MediaItem: Sendable, Codable, Hashable, Identifiable {
    public let id: UUID
    public var title: String
    public var category: MediaCategory
    public var status: MediaStatus
    public var rating: Int?              // 1...10, nil if unrated
    public var startedAt: Date?
    public var completedAt: Date?
    public var notes: String
    public var tags: [String]            // lowercased, deduped on save
    public var createdAt: Date
    public var updatedAt: Date
}
```

No `posterURL` / `externalID` in v1. Reserved for v2 metadata enrichment.

## Storage

`~/Library/Application Support/Luma/Media/media.json`

Schema versioning:

```json
{
  "version": 1,
  "items": [ ... MediaItem ... ]
}
```

Migration helper reads `version`; if missing, treats as v1. Future schema bumps follow the same pattern.

Size projection: an active user logging 200 items/year averages ~50 KB JSON for 5 years. SQLite migration only triggers above 5000 items, matching `docs/ROADMAP.md`'s policy on JSON-first persistence.

## Trigger conflict check

Existing single-letter triggers: `t` (Todo), `s` (Snippets). Adding `m` (Media) keeps three of four short slots used. Remaining single-letter slot: `b` (currently unused — reserved for Bookmarks if that ever ships).

Also accept the long form `media ` as a fallback, for users who type the full word.

## Stats (v1 scope, deliberately small)

Footer of detail view shows three numbers:

- Total items in current filter view
- Average rating (excludes unrated)
- Items marked Done this calendar year (excludes other statuses)

That's it. No charts, no heatmaps, no per-category breakdown. If you want richer stats, write them in your Notes vault, that's what it's for.

## Export

CSV with all fields (one row per item, RFC 4180). Filename: `luma-media-YYYY-MM-DD.csv`. Writes to `~/Downloads`. Triggered by the Export button in the detail toolbar.

No import in v1. Users moving from Douban can paste a Douban export CSV into the working JSON manually for now; an `Import CSV` command lands in v2 if there's actual demand.

## File structure

```
Sources/LumaModules/Media/
  MediaModule.swift              // handle / perform / capture vs search dispatch
  MediaStore.swift               // JSON load/save, validation
  MediaIndex.swift               // in-memory fuzzy search, frecency sort
  MediaParser.swift              // one-line DSL
  MediaActions.swift             // @MainActor bridge for opening edit sheet
Sources/LumaApp/Launcher/
  MediaDetailView.swift          // segmented + table + filter bar + edit sheet
Tests/LumaModulesTests/
  MediaParserTests.swift         // DSL coverage
  MediaIndexTests.swift          // ranking
  MediaStoreTests.swift          // persistence round-trip
```

Add `ModuleIdentifier.media = ModuleIdentifier(rawValue: "luma.media")` to `ModuleIdentifiers.swift`.

## Module manifest

```swift
public static let manifest = ModuleManifest(
    identifier: .media,
    displayName: "Media",
    capabilities: [.queryable, .providesActions],
    defaultEnabled: true,
    priority: 3,
    queryTimeout: .milliseconds(30)
)
```

Trigger keywords accepted by `handle`: `m ` and `media ` prefix. Anything else returns empty result.

## Hot-path discipline

- Warmup loads JSON into `MediaIndex` (in-memory).
- `handle` is pure memory lookup — never reads disk.
- Performance budget: `handle` p95 ≤ 20 ms warm. Add to `docs/specs/PERFORMANCE.md`.
- Edits write through `MediaStore.save()` and atomically swap the in-memory index.
- No network calls anywhere in v1.

## Tests

- **MediaParser**: each example in the DSL section becomes a unit test. Plus edge cases: leading numeric title (`1984`), bilingual title (`三体 / The Three-Body Problem`), rating-and-status ambiguity (`m foo 9` — rating 9, status default done).
- **MediaIndex**: fuzzy match on title, recency boost, category filter, status filter.
- **MediaStore**: round-trip (write → reload identical), schema version handling, malformed JSON recovery (returns empty + logs, never crashes).
- **MediaModule**: trigger extraction, capture vs search routing, action payload encoding.
- **Manual QA** (`docs/MANUAL_QA_CHECKLIST.md`):
  - Capture an item via `m`, dismiss panel, reopen, search — appears.
  - Edit via detail view, save, search — reflects edit.
  - Delete via Cmd+Delete in table — gone after restart.
  - Filter / sort combinations don't crash on empty result sets.
  - Export CSV opens correctly in Numbers.

## Acceptance criteria

- `swift test` green with new MediaParser/Index/Store tests.
- `m Oppenheimer movie 9` captures correctly and shows up in `m oppen` search.
- Detail view category tabs filter correctly; status filter and sort behave as labeled.
- 100 items load + first-keystroke render is under the 30 ms p95 ceiling.
- Deleting an item does not break the index for sibling items.
- Settings → Modules → "Media" toggle disables both trigger and detail view.

## Sequencing

| Step | Days | Deliverable |
| ---: | ---: | --- |
| 1 | 0.5 | Data model, store with round-trip tests |
| 2 | 1.0 | Parser + parser tests |
| 3 | 1.0 | Module + index + capture-vs-search dispatch + module tests |
| 4 | 1.5 | Detail view (segmented + table + filter bar) |
| 5 | 0.5 | Add/Edit sheet + delete + Esc/Return handling |
| 6 | 0.5 | Export CSV + footer stats |
| 7 | 0.5 | Manual QA, doc updates |

Total: ~5.5 days focused. Roughly the same as Snippets + Secrets put together, reflecting that Media has a richer detail view.

## Risks

1. **Detail-view scope creep**. Tabs → genre charts → annual wrap-ups → social sharing → Letterboxd. Mitigation: every detail-view feature request goes through ADR; default answer is "no, that's not what this module is."
2. **JSON growth**. At ~50 KB/year, fine for a decade. SQLite migration is an option, but premature now. Document the trigger threshold (5000 items) and move on.
3. **Trigger collision**: `m` is a common typo. Mitigation: require `m ` (with trailing space). Bare `m` returns empty, same as `s` and `t`.
4. **Bilingual titles**: Chinese title vs English title for the same work. v1: stored as one `title` field — the user picks which they want as the canonical name. v2 might add `titleAlt` for cross-lookup; defer until dogfooding shows it matters.
5. **Status semantics across categories**: "Reading" a book and "Watching" a TV show map to the same internal `inProgress` status. UI shows the verb appropriate to category in the popup. Internal model stays uniform.

## Docs to update on ship

- `docs/adr/011-media-tracker.md`: ADR documenting the trigger-only choice, the 5-category lock, and the non-goals listed above.
- `docs/ROADMAP.md`:
  - v0.1 (continuation) row: add Media v1 next to Snippets + Secrets.
  - Note that Media is trigger-only and does not consume a dashboard card slot.
- `docs/NON_GOALS.md`: append the items from the "What this module is not" section (Douban clone, streaming integration, posters, etc.).
- `docs/specs/PERFORMANCE.md`: Media `handle` p95 ≤ 20 ms warm; JSON load budget on warmup ≤ 80 ms for ≤ 5000 items.
- `docs/MANUAL_QA_CHECKLIST.md`: append the Media manual checklist items.

## Decisions locked

1. **Dashboard slot**: trigger-only. No card. (Option B confirmed 2026-06-22.)
2. **Bilingual title**: single `title` field in v1. `titleAlt` deferred to v2 pending dogfooding evidence.
3. **Single-letter triggers consumed**: `t` (Todo), `s` (Snippets, planned), `m` (Media). `b` reserved for future Bookmarks. No collisions.
4. **CSV export**: kept in v1 (half-day cost). Import deferred to v2.
5. **Metadata enrichment (TMDB / Open Library / Google Books)**: not in v1. Reconsider only after one month of v1 dogfooding shows the friction of typing titles is meaningful.

## v2 ideas (do not build until v1 ships and you've used it for a month)

- Optional TMDB / Open Library / Google Books metadata lookup on Add. Requires user-provided API key, network call out-of-band, poster cached to disk.
- Calendar / heatmap view of completions.
- Import from Douban CSV / Goodreads CSV / Steam export.
- `titleAlt` field for bilingual cross-search.
- Per-category bespoke stats (TV: shows in progress count; Games: backlog count).
- Quick-rating gesture: select a row and press 1-9 to set rating.
