# Luma Engineering Handbook

This is the primary engineering source of truth for Luma. It replaces the older split across PRD, architecture, specs, roadmap, non-goals, and integration notes.

## Product Shape

Luma is a personal, local-first macOS launcher for one keyboard-heavy developer.

- Native Swift 6 + AppKit primary UI.
- Default hotkey: Command+Space.
- Active UI route: Route C, command-first unified launcher.
- Empty-query home: Open Apps in the left column; command guide or module detail in the right column.
- Non-empty query: single result list, capped and stable.
- Module details stay in the launcher panel.
- Settings may use SwiftUI; launcher hot path stays AppKit.
- Current stage: finish, connect, and stabilize existing workflows. Do not add new product surface area by default.

## Architecture

```mermaid
flowchart TD
    Hotkey["HotkeyController"] --> Coordinator["AppCoordinator"]
    Coordinator --> Panel["LauncherWindowController + NSPanel"]
    Panel --> Root["LauncherRootView / LauncherRootController"]
    Root --> Home["LauncherHomeCoordinator"]
    Root --> VM["LauncherViewModel"]
    VM --> Dispatcher["QueryDispatcher actor"]
    Dispatcher --> Host["ModuleHost actor"]
    Host --> Modules["LumaModule actors"]
    Modules --> Services["LumaServices"]
    Modules --> Stores["Application Support / UserDefaults / Keychain"]
```

Layer ownership:

| Layer | Owns | Must not own |
| --- | --- | --- |
| `LumaApp` | App lifecycle, AppKit launcher, detail views, hotkey, settings | Module business logic |
| `LumaCore` | Protocols, models, query, ranking, actions, persistence helpers, design tokens | AppKit detail implementations |
| `LumaModules` | Built-in module actors, module stores/indexes, module actions | Launcher view hierarchy |
| `LumaServices` | System API wrappers for AX, CGWindow, EventKit, Keychain, AppleScript, processes | Product routing |
| `LumaInfrastructure` | Logging, metrics, configuration | User-facing flows |

Module detail views read shared module instances through `ModuleDetailRegistry` in `LumaApp`. Modules do not import or call the registry. Deprecated bridge APIs should not be used for new code.

## Launcher Contract

Home and panel behavior are frozen unless a new decision in `docs/DECISIONS.md` supersedes this section.

Home:

- Empty home shows only Open Apps on the left and a compact module guide on the right.
- Module detail replaces the guide on the right; Open Apps remains visible.
- The guide is read-only, not a second navigable list.
- Do not add setup rows, recent rows, project rows, create rows, dashboard cards, or suggestion sections to home.
- Open Apps lists regular running apps, excludes Luma itself, and may show child window rows for multi-window apps.
- Background Open Apps cache changes must not repaint the visible home list while the panel is open.
- Showing the launcher must not rebuild Open Apps for the first visible frame.

Panel:

- Default geometry: 940 x 760 pt.
- Responsive bounds: 720-980 pt wide, 640-820 pt high.
- Presentation screen is the screen with the active app, menu-bar context, key window, or cursor fallback.
- Show/hide may fade alpha; avoid geometry animation.
- Set frame atomically before ordering the panel front.
- Full-width content hosts must not use their own layer-backed root that changes anchor-point behavior. Put layer-backed chrome inside child views.
- Overlay hit testing is disabled during list/detail cross-fades.
- Text, buttons, table rows, and toolbars must not resize the panel or cause horizontal drift.

Keyboard:

| Key | Behavior |
| --- | --- |
| Esc | Action panel -> detail/results/home -> close |
| Return | Primary action for selected row |
| Command+Return | First secondary action |
| Tab or Command+K | Open action panel; close it when already open |
| Shift+Tab | Close action panel when open |
| Command+1...9 | Jump to result/action index where the active surface allows it |
| Arrow keys | Move list selection unless detail subview owns navigation |

In module detail, the search field is read-only and shows an "In <Module> -- Esc to go back" placeholder. Every detail exit path must restore search editability.

## Query And Module Contract

Modules implement `LumaModule`. Concrete modules should be actors unless stateless.

Required semantics:

- `manifest`: static metadata.
- `warmup`: load indexes and caches; soft budget 1 second.
- `handle`: answer from memory only; no disk, network, AppleScript, AX traversal, process enumeration, or large JSON parsing.
- `perform`: execute actions; soft budget 2 seconds via `ActionExecutor`.
- `teardown`: cancel background work and flush state.

Query rules:

- Global search requires at least 2 characters unless a command prefix is used.
- Bare commands open module detail or return a starter row as documented in `docs/MODULES.md`.
- Prefix search uses the selected module only.
- On-demand modules do not participate in global search unless explicitly documented.
- Disabled or permission-blocked modules return diagnostic rows, not silent empty results.
- Result IDs must be stable.
- Query tasks are cancelled on every new keystroke.
- Targeted cold modules may emit a warming or refreshing informational row.

Action rules:

- Panel hides before external actions complete.
- In-panel intents may keep the panel visible.
- Destructive persistent actions require confirmation or an undo/status path.
- Failures must surface a short status or diagnostic row.

## Performance Contract

Budgets:

| Metric | p95 target | Hard ceiling |
| --- | ---: | ---: |
| Hotkey -> interactive panel | 50 ms | 80 ms |
| Hotkey -> home painted | 50 ms | 80 ms |
| Keystroke -> first ranked paint | 30 ms | 60 ms |
| Module `handle` | Module timeout | 80 ms |
| Panel hide after action | 20 ms | 40 ms |

Hot path rules:

- Panel is pre-instantiated at app launch.
- Hotkey show reuses the already-rendered launcher/home UI.
- Empty persisted-session restore is a no-op for home rendering.
- Stale-while-revalidate is the default for slow system surfaces.
- `SelectionSnapshotService` may capture the frontmost PID on MainActor; AX IPC runs off-main.
- Browser Tabs must not await AppleScript on the keystroke path.
- Kill Process must not do process memory sampling on MainActor.
- Notes and Projects must query memory indexes, not scan disk.
- Menu Bar Search must query a cache, not traverse AX per keystroke.

Warmup:

- Pinned modules warm after startup.
- On-demand modules warm when targeted or opened.
- Warmup timeouts must not mark a module warm when the warmup did not complete.
- Fire-and-forget cache refreshes must still show a clear cold-cache state if the user queries before data arrives.

Memory:

- Idle teardown after hide should release on-demand module resources.
- Reserved modules and pinned modules are not torn down while they are expected to stay hot.
- Large stores should cap retained history and avoid loading unbounded data into detail views.

## Data And Privacy

Persistence:

- Application Support: JSON stores and module data.
- UserDefaults: lightweight settings.
- Keychain: secret values.
- EventKit: Todo reminders.
- Markdown files: Notes canonical content.

Privacy:

- No cloud sync, account, telemetry server, updater infrastructure, or public plugin marketplace in v1.
- Clipboard history has hard source/type/size/age filters.
- Secrets values are never exposed in global search.
- Browser Tabs is default-off because AppleScript and Automation prompts are sensitive.
- Accessibility permission is lazy: show the banner only on AX-dependent surfaces or after the user interacts with Open Apps window controls.

## Notes Format

Notes is a Markdown workspace manager, not an in-app editor.

- Markdown files and folders are the canonical store.
- `notes.json` is a local root/config/index helper, not the note database.
- Luma may read frontmatter, filenames, tags, wiki links, backlinks, and diagnostics.
- Typora or `NSWorkspace.open` owns editing/rendering.
- No proprietary note format, SQLite vault, multi-vault sync product, AI writing surface, or Obsidian-style graph product in v1.

`NotesRootConfig` schema v1 remains readable for at least three years. New fields should be optional; existing fields must not be repurposed.

## Non-Goals

- Cross-platform support.
- Electron, Tauri, or WebView primary UI.
- Public plugin API or plugin marketplace.
- Cloud sync, account system, analytics, or telemetry server.
- General theming beyond system light/dark.
- Dashboard home, home widgets, onboarding home, or home suggestion rows.
- Full-panel module detail that hides Open Apps on empty query.
- First-class note editor/renderer.
- Obsidian clone, required graph, or AI notes product.
- Notion-style Luma-owned todo database. Todo is EventKit pass-through.
- Full password manager, TOTP, browser autofill, or website-login vault.
- Media metadata enrichment, posters, streaming integration, social/discovery, or episode-level TV tracking.
- Kill Process daemon management or raw signal UI.

## Repo Map

```text
Sources/
  LumaApp/              App lifecycle, AppKit launcher, hotkey, settings
  LumaCore/             Protocols, models, query, actions, ranking, persistence, design tokens
  LumaModules/          Built-in modules and module stores/indexes
  LumaServices/         System API wrappers
  LumaInfrastructure/   Logging, metrics, configuration
Tests/
  LumaCoreTests/
  LumaModulesTests/
  LumaInfrastructureTests/
  LumaServicesTests/
docs/
  ENGINEERING.md        Current architecture, constraints, performance, non-goals
  MODULES.md            Current user-visible module behavior
  DECISIONS.md          Compact decision log
  QA.md                 Testing, manual QA, release
```

## Change Checklist

Before changing launcher home, panel layout, keyboard routing, module contracts, or hot-path behavior:

- Update this handbook and `docs/MODULES.md` if user-visible behavior changes.
- Add or update tests near the touched layer.
- Run `swift test`.
- For UI changes, run `./scripts/build_app.sh` and the relevant manual QA checks in `docs/QA.md`.
- Do not revive deleted historical behavior unless `docs/DECISIONS.md` records the new decision.
