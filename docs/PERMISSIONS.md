# Luma module permissions

Per-module permission and privacy matrix. **Default** matches manifest `defaultEnabled` and `ModuleWarmupDefaults` (D-012). MVP **P0 core** modules (Apps, Clipboard, Notes) are called out in `MVP_SCOPE.md`; Snippets, Quicklinks, and Translate are **Core P1 candidates** (default-on today); Todo follows an Open Decision.

| Module | Default | Permissions | Data accessed | Notes |
| --- | --- | --- | --- | --- |
| Apps | on | None for search | Installed apps index, running state (warmup cache) | `app top` uses memory cache from warmup |
| Clipboard | on | Accessibility (paste into apps) | Pasteboard history (filtered) | Sensitive types redacted |
| Commands | off | None | User `commands.json` scripts | Executables must live under `~/.luma/commands` |
| Menu Bar Search | off | Accessibility | Menu bar tree (cached) | Former doc name "Menu Items"; trigger `mb` / `menu` |
| Todo | on | Reminders | EventKit reminders | Auth cached at warmup; Open Decision per `MVP_SCOPE.md` |
| Wordbook | off | None | Local SQLite | |
| Notes | on | None | Notes root folder | |
| Browser Tabs | off | Automation (AppleScript) | Safari/Chrome tabs | |
| Window Layouts | off | Accessibility | Focused window geometry | Trust cached at warmup |
| Windows | deferred | Screen recording API | All window titles | **Not registered** in `ModuleRegistry.allBundles`; `BuiltInModules.makeDeferred()` only |
| Snippets | on | Accessibility (perform) | Snippet store | Core P1 candidate |
| Quicklinks | on | None | User links JSON | Core P1 candidate |
| Translate | on | Shortcuts / Translation | Typed text in perform | Bare `tr` opens detail; Core P1 candidate |
| Media | off | None | User media JSON | |
| Projects | off | None | Project scanner paths | |
| Kill Process | off | None | Process list (cached) | |
| Secrets | off | Keychain | Vault metadata only in search | Values never in handle |
| Workbench | parked | Clipboard / selection | Panel signals cache | **Not a `ModuleRegistry` module** — complex Workbench/Capture parked per `MVP_SCOPE.md` |

## Diagnostics

- `CrashLogBuffer` redacts at write time using the same rules as diagnostics export.
- Export path: `~/Library/Logs/Luma/diagnostics.json`

## Settings toggle

Disabling a module: tears down module actor, cancels active launcher query, closes detail if showing, evicts detail registry pool, invalidates panel signals and query snapshot cache.
