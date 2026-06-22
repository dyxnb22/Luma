# Dashboard Widget Cursor Implementation Plan

Status: implementation guide for Dashboard Widget Strategy  
Date: 2026-06-22  
Companion: `docs/strategy/DASHBOARD_WIDGET_STRATEGY.md`
Implementation status (2026-06-22): Phases 0 through 7 completed; Phase 8–10 polish underway per `docs/strategy/DASHBOARD_WIDGET_POLISH_PLAN.md`.

This document condenses the Cursor Composer-oriented implementation plan. Each phase should be executed independently and verified before moving on.

## Cursor Rules

- Give Cursor one prompt at a time.
- Do not let Cursor reinterpret product direction.
- Include exact files, exact constants, and exact acceptance checks.
- If Cursor adds unrelated features, stop and ask it to strictly follow the prompt.
- Run `swift build` after every phase.

## Phase 0: Remove Debug And Dead UI

### 0.1 Remove Visible Latency HUD

Files:

- `Sources/LumaApp/Launcher/LauncherRootView.swift`
- `Sources/LumaApp/Infrastructure/LatencyHUD.swift`

Tasks:

- Remove visible `LatencyHUD` from the panel.
- Keep telemetry as DEBUG log-only.
- Do not replace it with another visible debug control.

Acceptance:

- `swift build` passes.
- Panel no longer shows `p95 -- ms`.

### 0.2 Remove RecentItem Dead Code

Files:

- `Sources/LumaApp/Launcher/RecentItemProvider.swift`
- `Sources/LumaApp/Launcher/RecentItemButton.swift`
- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Tasks:

- Delete unused `RecentItemProvider` and `RecentItemButton`.
- Remove `recentStack`, `renderRecentItems`, and related setup.

Acceptance:

- `swift build` passes.
- No `RecentItem*` files remain.

## Phase 1: Panel And Glass Shell

### 1.1 LauncherPanel

File:

- `Sources/LumaApp/Launcher/LauncherPanel.swift`

Target values:

- `contentRect`: 860 x 540.
- `styleMask`: borderless, nonactivating panel, full size content view.
- `hasShadow = true`.
- `backgroundColor = .clear`.
- `isOpaque = false`.
- Keep key responder overrides.

Acceptance:

- `swift build` passes.
- Panel appears at 860 x 540.

### 1.2 LauncherRootView Glass Chrome

File:

- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Tasks:

- Make root view an `NSView`.
- Add `NSVisualEffectView` glass background with `.underWindowBackground`.
- Add top `CAGradientLayer` highlight.
- Add 20 pt continuous corner radius.
- Add 1 pt white alpha 0.18 border.

Acceptance:

- `swift build` passes.
- Panel has visible top highlight and thin inner border.

## Phase 2: Search Bar And Three-Column Layout

### 2.1 Add `LumaSearchBar`

File:

- `Sources/LumaApp/Launcher/LumaSearchBar.swift`

Requirements:

- Custom `NSView`.
- SF Symbol `magnifyingglass`.
- Borderless `NSTextField`.
- Placeholder: `Search`.
- Height: 52 pt.
- Keyboard callbacks for Esc, Up, Down, Tab, Command+1...9.

Acceptance:

- `swift build` passes.

### 2.2 Rebuild `LauncherRootView` Layout

File:

- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Required views:

- `searchBar`
- `sidebarContainer`
- `sidebarStack`
- `contentContainer`
- `featureGridView`
- `resultsScrollView`
- `resultsStackView`

Acceptance:

- Panel shows top search bar, left `OPEN APPS`, right empty content area.
- Sidebar width is 180 pt.
- Sidebar separator is visible.

## Phase 3: App Activation Tracker And Sidebar

### 3.1 Add `AppActivationTracker`

File:

- `Sources/LumaCore/Ranking/AppActivationTracker.swift`

Requirements:

- Record bundle ID, count, last activation date.
- Persist JSON to `~/Library/Application Support/Luma/app-activations.json`.
- Rank using 60% recency, 40% frequency.

Acceptance:

- `swift build` passes.

### 3.2 Connect Activation Tracking

Files:

- `Sources/LumaApp/App/AppCoordinator.swift`
- `Sources/LumaApp/Launcher/LauncherWindowController.swift`
- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Tasks:

- Listen to `NSWorkspace.didActivateApplicationNotification`.
- Record bundle activation.
- Pass tracker into launcher root view.
- Render up to 10 running apps in sidebar, active app highlighted.

Acceptance:

- `swift build` passes.
- Sidebar shows running apps.
- App order changes after usage.

## Phase 4: Widget Feature Grid

### 4.1 Add `WidgetFeatureCard`

File:

- `Sources/LumaApp/Launcher/WidgetFeatureCard.swift`

Requirements:

- 120 x 120 pt.
- 27 pt continuous corner radius.
- Vertical gradient background.
- SF Symbol, 48 pt.
- Title, 13 pt semibold.
- Command-number hotkey badge.
- Hover and press scale animations.

Acceptance:

- `swift build` passes.

### 4.2 Render Core Cards

Files:

- `Sources/LumaCore/Features/FeatureCard.swift`
- `Sources/LumaModules/FeatureCatalog.swift`
- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Core cards:

1. Translate
2. Clipboard
3. Calculator
4. Windows

Acceptance:

- `swift build` passes.
- Four gradient widget cards render in the main content area.
- Clicking a card opens or triggers the intended module behavior.

## Phase 5: Search Results

### 5.1 Connect Search To `LauncherViewModel`

File:

- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Tasks:

- Search text changes call `viewModel.queryChanged`.
- Non-empty query fades feature grid out and results list in.
- Empty query restores feature grid.
- Up/Down selection updates rows without full rebuild.
- Return executes selected item and hides panel.

Acceptance:

- `swift build` passes.
- Query shows results.
- Clear query restores cards.
- Keyboard navigation works.

### 5.2 Add `WidgetResultRow`

File:

- `Sources/LumaApp/Launcher/WidgetResultRow.swift`

Requirements:

- 56 pt height.
- 12 pt corner radius.
- 36 x 36 icon.
- 15 pt title, 12 pt subtitle.
- selected row background = accent alpha 0.22.
- selected row shows `↩`.

Acceptance:

- `swift build` passes.
- Result rows match widget dashboard spec.

### 5.3 Remove Old Result Row

File:

- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Task:

- Delete old `ResultItemRow` class and references.

Acceptance:

- `swift build` passes.

## Phase 6: Same-Panel Module Detail Views

### 6.1 Add Detail Protocol

File:

- `Sources/LumaApp/Launcher/ModuleDetailView.swift`

Requirements:

- `ModuleDetailView` protocol.
- `ModuleDetailRegistry`.
- Initial module IDs: translate, clipboard, calculator, windows.

### 6.2 Add Minimal Detail Views

File:

- `Sources/LumaApp/Launcher/ModuleDetailViews.swift`

Views:

- `TranslateDetailView`
- `ClipboardDetailView`
- `CalculatorDetailView`
- `WindowsDetailView`

Acceptance:

- `swift build` passes.

### 6.3 Connect Detail Navigation

File:

- `Sources/LumaApp/Launcher/LauncherRootView.swift`

Tasks:

- Add `detailContainer`.
- Add detail top bar with Back, title, x.
- Card click opens detail in content area.
- Back/Esc/x returns to feature grid.
- Sidebar and search bar remain visible.

Acceptance:

- Card -> detail works.
- Esc in detail returns to grid.
- Esc on grid closes panel.

## Phase 7: Polish

### 7.1 Fix Hide Flicker

File:

- `Sources/LumaApp/Launcher/LauncherWindowController.swift`

Task:

- Reset home state only after fade and `orderOut`.

Acceptance:

- No home flash during hide.

### 7.2 Hotkey Failure Visibility

Files:

- `Sources/LumaApp/App/MenuBarController.swift`
- `Sources/LumaApp/App/AppCoordinator.swift`

Task:

- Successful hotkey shows command icon.
- Failed hotkey shows warning icon and menu item telling user to disable Spotlight Command+Space.

### 7.3 Filter Luma Windows

File:

- `Sources/LumaModules/Windows/WindowsModule.swift`

Task:

- Filter windows whose pid matches `ProcessInfo.processInfo.processIdentifier`.

### 7.4 Clipboard Persistence

File:

- `Sources/LumaModules/Clipboard/ClipboardModule.swift`

Task:

- Ensure `ClipboardHistoryStore` uses `~/Library/Application Support/Luma/clipboard-history.json`.

## Full Acceptance Checklist

| # | Check |
| --- | --- |
| 1 | Panel has glass blur, top highlight, and thin inner border. |
| 2 | Sidebar shows running apps sorted by frecency. |
| 3 | Four widget cards render with gradients and hover scale. |
| 4 | Clicking a card opens same-panel module detail. |
| 5 | Typing search fades grid out and results in. |
| 6 | Up/Down, Command+1...9, Return, Esc all work. |
| 7 | Esc in detail returns to grid; Esc on grid closes panel. |
| 8 | Close/reopen starts from clean home with no flicker. |

## Status

- Phase 0 – Phase 7: completed.
- Phase 8 (state stability), Phase 9 (I/O coalescing), Phase 10 (consistency + docs): tracked in `docs/strategy/DASHBOARD_WIDGET_POLISH_PLAN.md`.

