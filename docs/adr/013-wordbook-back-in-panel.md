# ADR-013: Wordbook Back In Panel

## Status

Accepted. Supersedes ADR-009 §"Wordbook Review Panel (dedicated window)".

Date: 2026-06-22

## Context

ADR-009 placed Wordbook review in a dedicated `WordbookReviewPanel` NSPanel, separate from the 860×540 launcher. That violated Route B's core promise: **one window, module details in the same panel**.

The original rationale was that a 3–15 minute review session would "occupy" the launcher and violate the panel-hide p95 ≤ 20 ms budget. That concern does not hold:

- The launcher panel is pre-instantiated and resident; "occupying" it costs nothing when hidden.
- Other detail views (Notes, Translate, Clipboard) may also stay open for minutes without a separate window.
- Hot-path budget is Cmd+Space → first interactive ≤ 50 ms, independent of detail-view dwell time.

ADR-023 later superseded Route B home navigation. This ADR remains active for the Wordbook detail placement decision only: Wordbook review stays in-panel, but entry semantics now follow the Route C command-first unified list rather than dashboard cards or grid shortcuts.

## Decision

- Delete `WordbookReviewPanel.swift`.
- Add `WordbookDetailView: ModuleDetailView` in the main panel content area.
- Entry: `word` / `word review` query → "Start Review" result row → Return → in-panel detail.
- Exit: Esc → home list (same as Translate/Clipboard/Notes/Todo/Snippets/Secrets/Records).
- Review UI uses `BaseDetailContainer` with ~480 pt centered content width.
- Remove `LauncherBridge.openWordbookReview`; use `LauncherBridge.openModuleDetail(.wordbook)`.
- Grade shortcuts 1/2/3 and Space route through `ModuleDetailView.handleKeyDown` when the search field is empty.

## Consequences

Positive:

- True single-window experience; no visual or mental split for Wordbook.
- One fewer NSPanel class to maintain.
- Session restore can open Wordbook detail like any other module.

Negative:

- Review UI shares the launcher top bar (Back / title / ✕) instead of a minimal floating chrome.

## Performance

- `WordbookDetailView.activate()` budget: ≤ 100 ms warm, ≤ 200 ms cold (see `docs/specs/PERFORMANCE.md`).
