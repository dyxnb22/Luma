# Luma Features

This folder is the maintenance index for Luma's feature modules. Each feature owns a README with scope, data model, actions, privacy rules, UI behavior, and implementation notes.

## Active v1 Launcher Set (Route C — ADR-023)

**Home (frozen 2026-07-03):** Open Apps in the left column; right pane = command guide or module detail (ADR-032). No dashboard card grid, no setup/recent/continue/create sections, no `+N more`, no auto-onboarding.

**Modules (registered at launch):** Apps, Clipboard, Commands, Notes, Todo, Translate, Wordbook, Snippets, Secrets, Media, Window Layouts, Projects, Quicklinks, Menu Bar Search, Kill Process, Browser Tabs, Auto Workflow.

**Default off:** Commands, Media, Browser Tabs, Auto Workflow.

**Deferred:** Windows (source retained, not in `BuiltInModules.makeAll()`).

See `docs/adr/023-command-first-unified-list.md`, `docs/specs/LAUNCHER_HOME_CONSTRAINTS.md`, and `docs/ARCHITECTURE.md` for the current product surface.

## Feature Index

| Module | Default | Folder |
|--------|---------|--------|
| Apps | on | [AppSearch](AppSearch/) |
| Clipboard | on | [ClipboardHistory](ClipboardHistory/) |
| Commands | off | [CommandsScripts](CommandsScripts/) |
| Notes | on | [Notes](Notes/) |
| Todo | on | [Todo](Todo/) |
| Translate | on | [Translate](Translate/) |
| Wordbook | on | [Wordbook](Wordbook/) |
| Snippets | on | [Snippets](Snippets/) |
| Secrets | on | [SecretsVault](SecretsVault/) |
| Media (Records) | off | [Media](Media/) |
| Window Layouts | on | [WindowLayouts](WindowLayouts/) |
| Projects | on | [Projects](Projects/) |
| Quicklinks | on | [Quicklinks](Quicklinks/) |
| Menu Bar Search | on | [MenuItems](MenuItems/) |
| Kill Process | on | [KillProcess](KillProcess/) |
| Browser Tabs | off | [BrowserTabs](BrowserTabs/) |
| Auto Workflow | off | [Autoworkflow](Autoworkflow/) |

Deferred (source retained, not registered): Windows

Historical / superseded: [DashboardCards](DashboardCards/), [NotesGraph](NotesGraph/)

## Source Of Truth

Treat `BuiltInModules.swift`, `FeatureCatalog.swift`, and `docs/ARCHITECTURE.md` as source of truth for what ships today.

## Module Rules

- Every feature is represented as an independent `LumaModule` or service-backed module.
- Every default-enabled feature must preserve the launcher hot path.
- Empty-query home: **Open Apps** left column + guide/detail right (`LAUNCHER_HOME_CONSTRAINTS.md`, ADR-032).
- Sensitive features must keep secrets out of generic search results unless explicitly unlocked.

## Keyboard & Help (2026-07-02)

- **Search field** stays focused on open; ↑↓ moves selection; Return runs the row.
- **List area** accepts focus when clicked (padding/scrollbar); same keys work from the list.
- **Detail mode** clears the search query visually; Esc restores the suspended query.
- **Module help:** `help <trigger>` (IME-friendly) or `<trigger> ?` — see each module README.
- **Edit shortcuts:** Command+A/C/V/X/Z and undo/redo work in text fields via `LumaStandardEditShortcuts`.
- **Global search** needs ≥2 characters unless a command prefix is used.
- **i18n:** English + 简体中文 (`L10n` / `L10nStrings.json`); Settings → General → Language.
- **Translate replace selection:** Detail footer button; requires Accessibility; hides launcher before paste.

## Raycast-Inspired Defaults

- Fast command access and keyboard-first actions.
- Clipboard history with local retention and sensitive-content filtering.
- Window management through accessibility-backed commands.
- Lightweight note capture, but stored as plain Markdown files for Typora compatibility.
