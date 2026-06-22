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
- Floating Wordbook desktop-pet mode in v0.1. (Wordbook entry is the launcher card and `word ` trigger; see ADR-009.)
- ChatGPT-paste or CSV import UI for Wordbook in v0.1.

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
- Media tracker as a dashboard card (trigger-only by design; see ADR-011).
- Window tiling/layout engine as a v1 core feature.
- Dashboard-first launcher panel.
- Shortcut automation replacement.
- Complex onboarding tutorials.
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
- Inbox count, recent count, orphan count surfaced on the dashboard card.
