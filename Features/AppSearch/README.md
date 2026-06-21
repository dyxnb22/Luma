# App Search

## Goal

Provide a pinned Spotlight-like search box that can replace Spotlight for app launching. Typing an app name should show matching applications immediately; pressing Return or clicking a result opens the app directly.

## MVP Behavior

- Command+Space shows a pinned top search panel.
- Empty query shows frequent/recent apps and feature cards.
- Typing filters installed apps from `/Applications`, `/System/Applications`, and `~/Applications`.
- Clicking or pressing Return on an app result launches it with `NSWorkspace`.
- App search results should outrank most non-app results for short app-name queries.

## UI

- Spotlight/Raycast-like centered search panel.
- Rounded, translucent card surface.
- Search field pinned at the top.
- Results underneath with app icon, title, bundle id/path, and action hint.

## Implementation Entry

- Module: `Sources/LumaModules/Apps/AppsModule.swift`
- Pure index: `Sources/LumaModules/Apps/AppIndex.swift`
- UI shell: `Sources/LumaApp/Launcher`
