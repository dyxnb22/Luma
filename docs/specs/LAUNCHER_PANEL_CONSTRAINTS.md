# Launcher Panel Constraints (Frozen)

**Status:** Enforced as of 2026-07-03.  
**Authority:** Panel positioning, chrome, and in-panel layout. Complements [LAUNCHER_HOME_CONSTRAINTS.md](LAUNCHER_HOME_CONSTRAINTS.md) (home content freeze).

## Intent

The launcher is a **borderless, pre-instantiated floating panel** (Spotlight/Raycast proportions). It must stay **horizontally centered** on the presentation screen and **never shift or clip** when search results load, module prefixes appear, sectioned home/results render, or detail views open.

---

## Geometry (Frozen)

Authoritative tokens: `Sources/LumaCore/Design/LauncherChromeTokens.swift`  
Authoritative math: `Sources/LumaCore/Design/LauncherPanelGeometry.swift`

| Token / API | Value | Notes |
| --- | ---: | --- |
| `defaultPanelWidth` × `defaultPanelHeight` | **940 × 760** pt | Default on displays that fit (ADR-032) |
| `homeLeftColumnWidth` | **280** pt | Open Apps column on empty-query home |
| `minPanelWidth` / `maxPanelWidth` | 720 / 980 | Responsive clamp |
| `minPanelHeight` / `maxPanelHeight` | 640 / 820 | Responsive clamp |
| `panelVerticalBias` | **0.68** | Upper third of visible frame |
| `LauncherPanelGeometry.panelFrame(fitting:)` | — | Single source for size + origin |
| `LauncherPanel.position(on:)` | — | Atomic `setFrame`; locks `minSize`/`maxSize` |

Do **not** revert to the historical **900 × 600** wide shell without ADR.

---

## Presentation Screen (Frozen)

Use `LumaPresentationScreen.current()` (`Sources/LumaApp/UI/LumaPresentationScreen.swift`) before placing **any** transient Luma window:

1. Screen under the mouse cursor.
2. Else screen of `NSApp.keyWindow`.
3. Else `NSScreen.main` / first screen.

Applies to:

- Launcher show (`LauncherWindowController.positionPanel`)
- Settings show (`SettingsWindowController.centerOnPresentationScreen`)

---

## Positioning Rules (Do Not Break)

### Required

- Call `LauncherPanel.position(on:)` (or `LauncherPanelGeometry.panelFrame`) on **every** show — not only first launch.
- Use **one atomic** `setFrame(_:display:)` for origin + size together.
- Store `lockedFrameSize` on position; `setFrame` / `setContentSize` clamp to it — in-panel relayout must not widen the panel (regression: `help`, module prefixes).
- Re-center when the presentation screen may have changed (cursor moved to another display).
- Lock panel `minSize` and `maxSize` after positioning so internal Auto Layout cannot widen the window.
- After in-panel layout changes, call `LauncherInPanelLayout.stabilizePanel(from:)` → `enforceLockedGeometry()` + `stabilizeContentLayout()`.

### Forbidden

| Anti-pattern | Why |
| --- | --- |
| `contentView.layer?.anchorPoint = …` without compensating `position` | Shifts painted content inside the frame → horizontal clip (e.g. `clip` search) |
| `wantsLayer = true` on any **full-width** in-panel host | Default layer `anchorPoint (0.5, 0.5)` drifts when Auto Layout relayouts (command hint, section headers, detail open) |
| `CATransform3D` scale on the panel root for show/hide | Same drift risk; use alpha fade only |
| `setContentSize` then `setFrameOrigin` in separate steps without re-centering | Race / partial layout → off-center panel |
| `window.center()` for Settings on multi-monitor | Centers on wrong screen when cursor is elsewhere |
| Resizing the launcher panel from detail content | Detail scrolls inside fixed chrome; panel width is not content-driven |
| Relying on `minSize`/`maxSize` alone to block programmatic resize | AppKit may still `setFrame` wider during content fitting — clamp in `LauncherPanel.setFrame` |

### Show / hide animation

- **Allowed:** `alphaValue` fade (`MotionTokens.panelShowDuration` / `panelHideDuration`).
- **Not allowed:** scale transforms on window or root view layer.

---

## Technical Root Cause (Layer Drift)

AppKit views with `wantsLayer = true` get a backing layer whose default **`anchorPoint` is `(0.5, 0.5)`** (center). When a full-width host relayouts (hint bar appears, list sections reflow, detail toolbar grows), the layer-backed content is painted relative to that center anchor and can **shift horizontally** inside the fixed panel frame. The right edge then appears clipped even though the window did not resize.

**Fix pattern:** keep full-width hosts non-layer-backed (`clipsToBounds` only). Put glass, borders, selection fills, and shadows on a **child view pinned to all edges** of the host (or on small bounded widgets).

---

## In-Panel Content (All Modules)

Module surfaces share one fixed panel width. Content must **compress or scroll**, never expand the panel.

### Full-width hosts (no `wantsLayer`)

| Surface | File | Constraint |
| --- | --- | --- |
| `LauncherRootView` | `LauncherPanelChrome.swift` | No `wantsLayer`; glass + border on child `chromeOverlay` |
| `contentContainer` / `detailContainer` / `detailTopBar` | `LauncherLayoutBuilder.swift` | `clipsToBounds` only |
| `BaseDetailContainer` root | `GeekUIKit.installDetailRootChrome` | `clipsToBounds` only — all standard module details |
| `LauncherListRow` body (`NSControl`) | `LauncherListRow.swift` | No `wantsLayer` on row host; selection on `backgroundView` child |
| `LauncherListView` scroll view | `LauncherListView.swift` | `scrollView.clipsToBounds = true` |
| `CommandHintBar` / `LauncherHintBar` | respective files | Truncate long hints; no full-width layer |
| `LumaSearchBar` host | `LumaSearchBar.swift` | Leading/trailing pinned to root; chrome on child only |
| `PermissionBannerController.bannerView` | `PermissionBannerController.swift` | OK — chrome is pinned to child `chromeView`; no full-width host layer |

### Approved child-chrome helpers (`GeekUIKit`)

| Helper | Child identifier | Used by |
| --- | --- | --- |
| `installSearchSurface(on:)` | `geekSearchSurface` | `LumaSearchBar.surfaceView` |
| `installHomeListSurface(on:)` | (clear host) | Home list container |
| `installPerformanceStripSurface(on:)` | `performanceStripSurface` | Performance strip |
| `configureContentSurface(_:)` | `geekContentSurface` | `GeekGlassPanel`, detail section cards |
| `installSidebarAccent(on:)` | layer on chrome child | `LauncherListRow.backgroundView` only |
| `installDetailRootChrome(on:)` | — | `BaseDetailContainer` |
| `constrainDetailToolbarTrailingActions(_:in:after:)` | scroll wrapper | Crowded detail toolbars (Media) |

`configureGlassPanel` and `configureDetailTableRowSurface` may use `wantsLayer` only on **bounded** cards/cells, not panel-width hosts. `configureGlassPanel` on a panel-width host is a legacy footgun — prefer `configureContentSurface`.

### Custom detail views

| Module | File | Width constraint |
| --- | --- | --- |
| Translate | `TranslateDetailView.swift` | `panelsStack`, `toolbar`, `errorBanner` pin to `container` leading/trailing |
| Notes | `NotesDetailView.swift` | `topStrip`, `chipBar`, `filterStrip`, scroll views pin to `container` — IA: [NOTES_DETAIL_CONSTRAINTS.md](NOTES_DETAIL_CONSTRAINTS.md) |

Custom detail toolbars with many controls: use `GeekUIKit.constrainDetailToolbarTrailingActions(_:in:after:)` — trailing actions scroll inside fixed width, do not grow the panel. **Required** for crowded toolbars.

### Overlay hit-testing (detail / list cross-fade)

During `detailContainer` ↔ `listView` alpha animations, views at `alphaValue = 0` still receive clicks unless `isHidden` or `ignoresMouseEvents`. **Required:** disable mouse hits on the fading overlay until the transition completes (regression risk: list unclickable mid-fade — audit L1).

### Panel stabilization

`LauncherInPanelLayout.stabilizePanel(from:)` → `LauncherPanel.enforceLockedGeometry()` (clamp size to `lockedFrameSize` + re-center) + `stabilizeContentLayout()`.

Call after:

| Transition | Caller |
| --- | --- |
| Search text / prefix change | `LauncherRootController` |
| Results paint / home restore | `LauncherContentCoordinator` |
| Detail open / close | `LauncherContentCoordinator` |

### Module audit (`wantsLayer` risk)

| Module / surface | Host pattern | Status |
| --- | --- | --- |
| Home list | `installHomeListSurface` → clear host | OK |
| Search bar | `installSearchSurface` → child `geekSearchSurface` | OK |
| List rows | `backgroundView` child for selection; row body no layer | OK |
| Command / hint bars | Truncate; no full-width layer | OK |
| Clipboard, Todo, Secrets, Snippets, Media, Wordbook, Projects, Quicklinks, Current Project | `BaseDetailContainer` → `installDetailRootChrome` | OK |
| Translate | `GeekGlassPanel` + `configureContentSurface` chrome child | OK |
| Notes | Custom detail; stacks pinned to `container` | OK |
| Action panel (Tab / ⌘K) | `LauncherActionPanel.chromeView` child | OK |
| Permission banner | `PermissionBannerController.bannerView` | OK — chrome is pinned to `chromeView` |
| Detail toolbars (Clipboard, Secrets, Todo) | Horizontal button stacks | OK — use horizontal overflow constraints where crowded |
| Bounded widgets (todo row, clip thumbnail, keycaps, badges, table rows, icons) | Fixed-size cells — layer on widget only | OK — do not layer full-width parents |

**Rule:** Never set `wantsLayer = true` on a view whose width equals the launcher panel (`LauncherRootView`, `contentContainer`, `detailContainer`, list row host, `BaseDetailContainer` root, custom detail root). Put glass/border/selection on a **pinned child** instead.

---

## Home split layout (ADR-032)

When the visible search query is **empty**:

| Region | Width | Content |
| --- | --- | --- |
| Left column | `homeLeftColumnWidth` (280 pt) | Open Apps list |
| Right column | Remainder (~620 pt on default panel) | Command guide or module detail |

- Active search (non-empty query) → **single-column** results; split off.
- Module detail **never** uses full-panel overlay when visible query is empty.
- `detailContainer` constraints: `LauncherHomeSplitLayout` right-column anchors only.
- All module detail views must pin to `detailContainer` width and scroll — same rules as full-width detail, narrower canvas.

Implementation: `LauncherHomeSplitLayout.swift`, `LauncherHomeGuidePane.swift`, `LauncherRootController.syncSplitLayout()`.

---

## New Module Detail Checklist

When adding or changing a module detail view in `Sources/LumaApp/Launcher/`:

1. Prefer `BaseDetailContainer` + `GeekUIKit.installDetailRootChrome` — do not add `wantsLayer` on the container root.
2. If building a custom root (`TranslateDetailView` style), pin every horizontal stack to `container.leadingAnchor` / `trailingAnchor` or `container.widthAnchor`.
3. Use `GeekGlassPanel` or `configureContentSurface` for cards — never call `configureContentSurface` on a panel-width parent without the built-in chrome child.
4. Do not change `LauncherChromeTokens` panel size tokens from detail layout.
5. Ensure the coordinator path calls `stabilizePanel` after your detail opens (wire through existing `LauncherContentCoordinator` hooks).
6. Manual QA: type the module prefix (`clip`, `note`, `tr`, …) before and after opening detail — no horizontal drift or right-edge clip.

---

## Code Touchpoints

| Area | File | Constraint |
| --- | --- | --- |
| Panel frame math | `LauncherPanelGeometry.swift` | Only place for size/bias formulas |
| Panel show / stabilize | `LauncherPanel.swift` | `position(on:)`, `lockedFrameSize`, `enforceLockedGeometry`, `stabilizeContentLayout` |
| Window controller | `LauncherWindowController.swift` | `positionPanel()` before `orderFront`; `panelContentHost` wrapper |
| Presentation screen | `LumaPresentationScreen.swift` | Shared screen selection |
| Settings placement | `SettingsWindowController.swift` | `centerOnPresentationScreen` on every present |
| Root chrome | `LauncherPanelChrome.swift` | Glass + border on child overlay |
| Layout builder | `LauncherLayoutBuilder.swift` | No `wantsLayer` on full-width containers; **no** `listView.trailing` — split layout owns horizontal sizing |
| In-panel stabilize | `LauncherInPanelLayout.swift` | Single entry for post-layout stabilize |
| Content coordinator | `LauncherContentCoordinator.swift` | Right-column detail; `stabilizePanel` on present/close |
| Home split | `LauncherHomeSplitLayout.swift` | Column split + right pane guide/detail |
| Root controller | `LauncherRootController.swift` | `syncSplitLayout` on home/detail/search transitions |
| Shared styling | `GeekUIKit.swift` | Child-chrome helpers above |
| Detail chrome | `BaseDetailContainer.swift` | `installDetailRootChrome` |
| Tokens | `LauncherChromeTokens.swift` | Default 940×760, home left 280pt, bias 0.68 |
| Tests | `LauncherPanelGeometryTests.swift` | Geometry math regression |

---

## PR Checklist (Panel / Chrome Changes)

Any PR that touches panel positioning, size tokens, presentation screen, launcher root layout, or in-panel `wantsLayer` must:

1. Cite an ADR if changing default geometry or show animation model.
2. Update this file and `docs/specs/UX_BEHAVIOR_RULES.md` Panel section.
3. Update `docs/MANUAL_QA_CHECKLIST.md` Panel / in-panel layout sections.
4. Update `.cursor/rules/launcher-panel-chrome.mdc` if rules change.
5. Add/adjust tests in `LauncherPanelGeometryTests` when math changes.

---

## Manual QA (Quick)

- Open launcher on **each** connected display (move cursor first) → panel centered, not clipped on the right.
- Type module prefixes — panel stays centered; search field and list align with panel edges:
  - `help`, `?`, `clip`, `note`, `tr`, `word`, `rec`, `t`, `secret`, `ql`, `proj`
- `help` / `?` global command list must not expand panel width — rows truncate inside fixed width.
- Open Translate / Notes / Clipboard detail → no horizontal growth; toolbars scroll if crowded.
- Tab action panel on a result → overlay does not shift the panel.
- Open Settings from menu while cursor is on secondary display → window appears on that display.

---

## Related Docs

- [LAUNCHER_HOME_CONSTRAINTS.md](LAUNCHER_HOME_CONSTRAINTS.md) — empty-query home freeze
- [UX_BEHAVIOR_RULES.md](UX_BEHAVIOR_RULES.md) — interaction contract
- [LAUNCHER_NAVIGATION_AUDIT.md](../qa/LAUNCHER_NAVIGATION_AUDIT.md) — temporary open-issue register (navigation, shortcuts, session)
- [MANUAL_QA_CHECKLIST.md](../MANUAL_QA_CHECKLIST.md) — regression checks
- [ADR-023](../adr/023-command-first-unified-list.md) — Route C + panel geometry note
- [ADR-002](../adr/002-preinstantiated-panel.md) — pre-instantiated panel model
- [MODULE_CONTRACT.md](MODULE_CONTRACT.md) — module detail UI boundary
