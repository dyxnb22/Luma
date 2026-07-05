# Snippets

## Goal

Fast text expansion from the launcher. Store reusable text blocks, paste them by name, or trigger them inline with a keyword shortcut.

## Triggers

- `s` / `snip` — open Snippets detail
- `s <query>` — search snippets by title or tag
- `snip new <title>` — shortcut to open snippet creation with prefilled title
- typing an exact trigger word in global search + Return → inline expansion (no detail)

## Data Model

Stored in `~/Library/Application Support/Luma/snippets.json`.

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | stable row identity |
| `title` | String | display name, searchable |
| `content` | String | body; may contain variables |
| `trigger` | String | optional keyword for inline expansion |
| `tags` | [String] | optional filter labels |
| `usageCount` | Int | drives recency ranking |
| `createdAt` | Date | |

## Variable Expansion

`SnippetVariableExpander` resolves variables at paste/expand time:

- `{{date}}` — today's date
- `{{time}}` — current time
- `{{clipboard}}` — current clipboard text
- `{{selection}}` — selected text at launcher activation
- `{{project}}` — current project name from `CurrentProjectService`

## Actions

- **Copy** (Return) — copies expanded content to pasteboard
- **Paste** (Command+Return) — copies + AX-inserts into frontmost app; requires Accessibility permission
- **Action panel** (Tab or ⌘K) — choose Copy or Paste; Return runs the highlighted action
- **Inline trigger expansion** — if raw query exactly matches `trigger` (case-insensitive) in global search, Return expands and pastes without opening detail
- **Edit / Delete / Duplicate** — via detail view; changes persist immediately

## Privacy Rules

Snippet content is local-only. No content leaves the device. `SnippetsStore` never writes to iCloud or any network location.

## Permissions

- **Accessibility** required for Paste (AX `insert`). Copy works without it.
- If AX is denied, Paste shows a status hint; Copy still works.

## Warmup

`warmup` loads all snippets into `cachedSnippets: [Snippet]`. Subsequent `handle` calls are memory-only (no disk I/O per keystroke). Phase 1.

## Implementation Entry

- Module: `Sources/LumaModules/Snippets/SnippetsModule.swift`
- Store: `Sources/LumaModules/Snippets/SnippetsStore.swift`
- Variable expansion: `Sources/LumaModules/Snippets/SnippetVariableExpander.swift`
- Trigger expansion intercept: `Sources/LumaApp/Launcher/LauncherRootController.swift` → `activateReturn`
