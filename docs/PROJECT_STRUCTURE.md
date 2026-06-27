# Project Structure

```text
Sources/
  LumaApp/              App lifecycle, AppKit launcher, hotkey
  LumaCore/             Protocols, models, query, actions, ranking, persistence
  LumaModules/          Built-in modules; active set includes Apps, Clipboard, Commands, Notes, Todo, Translate, Wordbook, Snippets, Secrets, Media, Window Layouts, Projects, Quicklinks, Menu Bar Search, Kill Process, Browser Tabs (optional)
  LumaServices/         System API wrappers
  LumaInfrastructure/   Logging, metrics, configuration
Tests/
  LumaCoreTests/
  LumaModulesTests/
docs/
  adr/
  specs/
  planning/
.claude/
.cursor/
.codex/
```

Rules:

- `LumaCore` stays mostly platform-model oriented and avoids owning UI.
- `LumaApp` owns AppKit.
- `LumaModules` contains feature folders; each module owns its index and store.
- `LumaServices` isolates macOS APIs that are hard to test directly.
- `LumaInfrastructure` provides boring cross-cutting utilities.
