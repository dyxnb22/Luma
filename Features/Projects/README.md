# Project Switcher

## Goal

Quickly open frequently used dev projects in Cursor, VS Code, Finder, or Terminal. Command-first — not an IDE project manager.

## Active Behavior (Route C)

- Registered through `ModuleRegistry.allBundles` and instantiated by `BuiltInModules.makeAll()`.
- Prefix triggers: `proj`, `p`, `project` (e.g. `proj luma`, `p luma`).
- Empty payload: up to 8 pinned/recent projects.
- With query: fuzzy search name, path, aliases from in-memory index.
- Primary action: open with `preferredOpener`; Tab / ⌘K for other openers, Copy Path, Reveal Config.
- **No disk scan on query** — index built at warmup only.

## Configuration

`~/Library/Application Support/Luma/projects.json`:

```json
{
  "roots": ["~/Developer", "~/Projects"],
  "projects": [
    {
      "name": "Luma",
      "path": "/Users/you/Luma",
      "aliases": ["luma"],
      "preferredOpener": "cursor",
      "pinned": true
    }
  ],
  "recent": []
}
```

Warmup shallow-scans `roots` (depth 2, ~400 ms budget) for `.git`, `Package.swift`, `package.json`, `pyproject.toml`, `Cargo.toml`, `*.xcodeproj`, `*.xcworkspace`.

## Implementation Entry

- `Sources/LumaModules/Projects/ProjectsModule.swift`
- `Sources/LumaModules/Projects/ProjectStore.swift`
- `Sources/LumaModules/Projects/ProjectScanner.swift`
- `Sources/LumaModules/Projects/ProjectIndex.swift`

## Out of Scope (v1)

- Settings UI for roots/projects
- Pin/unpin from launcher (config file only)
- Home-screen recent projects section
