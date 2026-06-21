# Roadmap

## Phase 0: Skeleton

- Swift package and eventual Xcode project wiring.
- LSUIElement/accessory app setup.
- Carbon hotkey controller with Command+Space as the default.
- Menu bar item with Show Luma, Settings..., and Quit Luma.
- Pre-instantiated empty panel.
- Metrics hooks.
- `Features/` folder with per-module docs.

## Phase 1: Launcher and Card Shell

- `LauncherPanel`, `QueryField`, `ResultListView`, `ResultCellView`.
- Spotlight/Raycast-like AppKit visual language.
- Rounded card dashboard with edit buttons.
- Drag-to-reorder card position model.
- Keyboard navigation.
- Fake static results.
- Snapshot diffing.
- Gate: fake keystroke -> paint p95 <= 30 ms.

## Phase 2: Core Data and Ranking

- Fuzzy matcher and ranker.
- Usage tracker.
- GRDB integration.
- Feature card layout persistence.
- Apps and Commands modules.
- First daily-dogfood build.

## Phase 3: Raycast-Like Workflows

- Translate module.
- Clipboard history module with secret filtering.
- Clipboard retention caps: 500 entries, 7 days, 100 KB per entry.
- Window Layouts module.
- Lazy Accessibility permission flow.

## Phase 4: Personal Knowledge and Secrets

- Secrets Vault with Keychain-backed encryption direction.
- Notes Graph module.
- Markdown vault tree.
- Typora open/edit integration.
- Backlink/tag graph index.

## Phase 5: Wordbook Migration

- Import `/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3`.
- Preserve 9-stage review schedule.
- Review card with known/fuzzy/unknown actions.
- macOS speech for word/example.
- Daily goal/progress card.

## Phase 6: Polish and Release

- Settings.
- Multi-monitor/fullscreen Spaces verification.
- Developer ID signed and notarized DMG.
- Latency regression checklist.
- Local release script.

## Deferred

- Public plugin API.
- Cloud sync.
- Notes editor replacement.
- Complex animated graph rendering before the index is solid.
