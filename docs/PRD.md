# Luma PRD

## Summary

Luma is a personal macOS launcher and workbench for one keyboard-heavy developer. It opens via Command+Space, provides Raycast-like command speed, Spotlight-like visual calm, and a rounded-card dashboard for modular personal workflows.

## Product Goals

- Native AppKit launcher with p95 hotkey-to-interactive <= 50 ms.
- Modular features that can be enabled, disabled, edited, and reordered.
- Card-based management UI with draggable positions and edit buttons.
- Local-first data for clipboard, secrets, notes, and word review.
- Plain Markdown notes compatible with Typora.
- Wordbook migration from `/Users/diaoyuxuan/wordbot`.

## Required Features

1. Translate selected, clipboard, or typed text.
2. Clipboard history with local retention, pin/delete/clear, and strict sensitive-content filtering.
3. Secrets Vault for passwords, API keys, tokens, recovery codes, and key information.
4. Window/page layout shortcuts similar to Raycast window management.
5. Notes Graph: Obsidian-like Markdown vault, folder tree, backlinks, tags, and graph index.
6. Wordbook: migrate native functionality from `/Users/diaoyuxuan/wordbot`.
7. Spotlight/Raycast-like UI with macOS 26/iOS 26-style rounded cards.
8. Dashboard cards with edit buttons and drag-to-reorder position management.
9. Modular management for every feature.
10. `Features/` folder containing per-module introductions and maintenance notes.

## Wordbook Requirements

- Import existing SQLite data from `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- Preserve the 9-stage Ebbinghaus-style schedule.
- Support known/fuzzy/unknown review responses.
- Show daily goal/progress and due count.
- Use macOS speech for word/example pronunciation.

## Success Criteria

- Hotkey -> visible interactive panel: p95 <= 50 ms.
- Keystroke -> first ranked snapshot painted: p95 <= 30 ms.
- Adding or maintaining a feature starts from its folder in `Features/`.
- Every user-facing feature has a `LumaModule` boundary.
- Dashboard card layout persists after drag/reorder.

## Platform

- macOS 14+.
- Swift 6 strict concurrency.
- AppKit for launcher and dashboard.
- SwiftUI allowed for Settings/About only.
- Developer ID signed and notarized DMG for v1.
