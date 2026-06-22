# Luma Features

This folder is the maintenance index for Luma's feature modules. Each feature owns a README with scope, data model, actions, privacy rules, UI behavior, and implementation notes.

## Active v1 Launcher Set

1. App Search / Launcher
2. Window Focus
3. Clipboard History
4. Translate
5. Frecency Recent Items
6. Quick Calculator

See `docs/strategy/LAUNCHER_CONVERGENCE_STRATEGY.md` for the current strategic rationale.

## Deferred Or Experimental Features

- Dashboard Cards
- Notes Graph
- Wordbook
- Secrets Vault
- Window Layout engine

These docs may remain as historical or experimental references, but they should not define the v1 launcher path.

## Module Rules

- Every feature is represented as an independent `LumaModule` or service-backed module.
- Every default-enabled feature must preserve the launcher hot path.
- Empty-query UI is usage/frecency-based, not card-based.
- Sensitive features must keep secrets out of generic search results unless explicitly unlocked.

## Raycast-Inspired Defaults

- Fast command access and keyboard-first actions.
- Clipboard history with local retention and sensitive-content filtering.
- Window management through accessibility-backed commands.
- Lightweight note capture, but stored as plain Markdown files for Typora compatibility.
