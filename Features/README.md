# Luma Features

This folder is the maintenance index for Luma's feature modules. Each feature owns a README with scope, data model, actions, privacy rules, UI behavior, and implementation notes.

## Active v1 Launcher Set (Route C — ADR-023)

**Home sections:** Open Apps, Suggested (no dashboard card grid).

**Modules (registered at launch):** Apps, Clipboard, Commands, Notes, Todo, Translate, Wordbook, Snippets, Secrets, Media, Window Layouts, Projects.

**Deferred:** Windows (source retained, not in `BuiltInModules.makeAll()`).

See `docs/adr/023-command-first-unified-list.md` and `docs/ARCHITECTURE.md` for the current product surface.

## Historical / Experimental Docs

Older per-feature READMEs under this folder may describe pre–Route B experiments. Treat `BuiltInModules.swift`, `FeatureCatalog.swift`, and `docs/ARCHITECTURE.md` as source of truth for what ships today.

## Module Rules

- Every feature is represented as an independent `LumaModule` or service-backed module.
- Every default-enabled feature must preserve the launcher hot path.
- Empty-query UI shows a sectioned home list (Open Apps / Suggested).
- Sensitive features must keep secrets out of generic search results unless explicitly unlocked.

## Raycast-Inspired Defaults

- Fast command access and keyboard-first actions.
- Clipboard history with local retention and sensitive-content filtering.
- Window management through accessibility-backed commands.
- Lightweight note capture, but stored as plain Markdown files for Typora compatibility.
