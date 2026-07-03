# Non-Goals

- Cross-platform support.
- Electron, Tauri, or WebView-based primary UI.
- Public plugin API or plugin marketplace in v1.
- Account, cloud sync, analytics, or telemetry server.
- General-purpose theming beyond system light/dark support.
- Custom file index.
- First-class note-taking app.
- Obsidian-style graph/backlink product.
- First-class browser password autofill or website-login manager.
- Developer credential vault (Secrets module) covers API keys and similar; see ADR-010.
- TOTP / 2FA code generation in v1.
- Notion-style Luma-owned TODO database. (Todo is an EventKit pass-through; see ADR-009.)
- Floating Wordbook desktop-pet mode in v0.1. (Wordbook entry is through the launcher and `word` trigger; see ADR-009.)
- ChatGPT-paste or CSV import UI for Wordbook in v0.1. (CSV import/export shipped; see ADR-020.)

## Launcher Home (Frozen — do not restore without ADR)

Authoritative spec: `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`.

- Setup / Get Started section or setup rows on empty-query home.
- Recent, Continue, or Create suggestion sections on home.
- `+N more` collapsed Open Apps row.
- Auto-present first-run onboarding wizard on launch.
- Gray card background on every idle list row (reads as disabled UI).
- Dashboard card grid or permanent module sidebar on home.

## Launcher Panel (Frozen — do not change without ADR)

Authoritative spec: `docs/specs/LAUNCHER_PANEL_CONSTRAINTS.md`.

- Reverting to **900 × 600** wide-dashboard panel proportions.
- `wantsLayer` or scale transforms on full-width in-panel hosts (root, list container, detail root).
- Resizing the panel from detail content width.
- `window.center()` for transient windows on multi-monitor setups.

## Wordbook Module

- Multi-book / multi-deck / multi-language switching.
- Custom SRS intervals or alternative algorithms (FSRS, SM-2).
- AI-generated definitions, examples, translations, or word roots.
- Online dictionary lookup beyond opt-in IPA (`dictionaryapi.dev`).
- Dictation, spelling practice, or non–three-button card types.
- Custom card front/back templates.
- Learning curves, heatmaps, or long-range statistics dashboards.
- Review push notifications or proactive menu-bar badges.
- Floating desktop word-pet mode (ADR-009).
- Cloud sync, accounts, or cross-device replication.
- Category rename/merge/delete batch tools (`category` is a free-form filter tag only).
- Separate label/tag system beyond `category`.
- PDF/EPUB import (CSV/TSV only).

Reference: ADR-020 Wordbook v1 frozen scope.

## Clipboard Module

- OCR on clipboard images.
- Cloud sync, accounts, or multi-device clipboard.
- Pinboard / multiple named boards.
- Rich-text editor or stored RTF payloads.
- File previewer or color palette / library UI.
- Entry rename, multi-select bulk queue, complex per-rule editors.
- SQLite or full-text index (linear search up to retention cap).
- Grid / card-wall history UI (Paste-style).

Reference: ADR-021 Clipboard v1 frozen scope.

## Media Module

- Douban / Letterboxd / Trakt-style social, comments, friend feeds, public profiles.
- Discovery / recommendations of what to watch next.
- Streaming integration (Netflix activity, Spotify history, Steam library import).
- Poster / cover-art fetching or caching in v1.
- Review writing as anything beyond plain text. No markdown rendering, no rich text.
- Episode-level tracking for TV (one status per series, not per-episode).
- Time-played tracking for games, achievements, trophies, completion percentages.
- Per-rewatch / per-replay separate records. Rewatching updates `completedAt`.
- AI summarisation or auto-generated reviews.
- Cloud sync, sharing, multi-device.
- Metadata enrichment (TMDB / OMDb / Open Library / Google Books / RAWG) in v1.
- Media tracker as a home-screen card or dashboard surface.
- Window tiling/layout engine as a v1 core feature.
- Dashboard-first launcher panel.
- Home suggestion / continue / create rows on empty query (use search and commands).
- Shortcut automation replacement.
- Auto-present onboarding wizard on first launch.
- Heavy architecture frameworks unless a concrete local pain justifies them.

## Notes Module

- Page embedding / transclusion / `![[Note]]` rendering.
- Wiki-link backlinks, forward-link graphs, graph view, orphan dashboards beyond the image utility.
- Tags, frontmatter parsing, frontmatter generation, templates.
- Markdown rendering of any kind. Markdown editing of any kind.
- Full-text search of note bodies. Filename and folder name search only.
- AI summarisation, AI Q&A, semantic search.
- Multi-vault, vault sync, vault import/export, version history.
- Image insertion / paste interception / image gallery / image compression / format conversion.
- Drag-to-move tree nodes.
- Inbox count, recent count, orphan count surfaced on the launcher home.
