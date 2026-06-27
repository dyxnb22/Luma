# App Search

## Goal

Provide fast app launching inside the Route C unified list. App results should feel instant for short queries and remain useful for mixed-language names, aliases, and abbreviations.

## Active Behavior (Route C)

- Command+Space opens the launcher panel.
- Empty query shows the sectioned home list, including Open Apps and Suggested.
- Non-empty query searches the app index alongside other enabled modules.
- Typing filters indexed apps from `/Applications`, `/System/Applications`, and `~/Applications`.
- Clicking or pressing Return on an app result launches or activates it with `NSWorkspace`.
- App search should outrank most non-app results for short app-name queries.
- Fuzzy matching supports localized names, aliases, pinyin, and subsequence matching per ADR-015.

## UI

- Spotlight/Raycast-like centered launcher panel.
- Search field pinned at the top.
- Empty query uses the shared home list, not feature cards.
- Results render in the shared unified list with app icon, title, path/bundle metadata, and action hint.

## Implementation Entry

- Module: `Sources/LumaModules/Apps/AppsModule.swift`
- Pure index: `Sources/LumaModules/Apps/AppIndex.swift`
- UI shell: `Sources/LumaApp/Launcher`
