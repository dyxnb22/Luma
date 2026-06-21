# Luma Features

This folder is the maintenance index for Luma's feature modules. Each feature owns a README with scope, data model, actions, privacy rules, UI card behavior, and implementation notes.

## v1 Feature Set

1. Translate
2. Clipboard History
3. Secrets Vault
4. Window Layouts
5. Notes Graph
6. Wordbook
7. Dashboard Cards

## Module Rules

- Every feature is represented as an independent `LumaModule` or service-backed module.
- Every feature can be enabled, disabled, and reordered from the card dashboard.
- Every feature card exposes an edit button.
- Dashboard cards are draggable and persist their position.
- Sensitive features must keep secrets out of generic search results unless explicitly unlocked.

## Raycast-Inspired Defaults

- Fast command access and keyboard-first actions.
- Clipboard history with local retention and sensitive-content filtering.
- Window management through accessibility-backed commands.
- Lightweight note capture, but stored as plain Markdown files for Typora compatibility.
