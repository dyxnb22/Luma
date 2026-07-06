# Luma module permissions

Per-module permission and privacy matrix. Modules default-off unless noted.

| Module | Default | Permissions | Data accessed | Notes |
| --- | --- | --- | --- | --- |
| Apps | on | None for search | Installed apps index, running state (warmup cache) | `app top` uses memory cache from warmup |
| Clipboard | on | Accessibility (paste into apps) | Pasteboard history (filtered) | Sensitive types redacted |
| Commands | off | None | User `commands.json` scripts | Executables must live under `~/.luma/commands` |
| Menu Items | on | Accessibility | Menu bar tree (cached) | |
| Todo | on | Reminders | EventKit reminders | Auth cached at warmup |
| Wordbook | on | None | Local SQLite | |
| Notes | on | None | Notes root folder | |
| Browser Tabs | off | Automation (AppleScript) | Safari/Chrome tabs | |
| Window Layouts | on | Accessibility | Focused window geometry | Trust cached at warmup |
| Windows | deferred | Screen recording API | All window titles | Not registered by default |
| Snippets | on | Accessibility (perform) | Snippet store | |
| Quicklinks | on | None | User links JSON | |
| Translate | on | Shortcuts / Translation | Typed text in perform | Bare `tr` opens detail |
| Media | on | None | User media JSON | |
| Projects | on | None | Project scanner paths | |
| Kill Process | on | None | Process list (cached) | |
| Secrets | on | Keychain | Vault metadata only in search | Values never in handle |
| Workbench | on | Clipboard / selection | Panel signals cache | Preview is side-effect free |

## Diagnostics

- `CrashLogBuffer` redacts at write time using the same rules as diagnostics export.
- Export path: `~/Library/Logs/Luma/diagnostics.json`

## Settings toggle

Disabling a module: tears down module actor, cancels active launcher query, closes detail if showing, evicts detail registry pool, invalidates panel signals and query snapshot cache.
