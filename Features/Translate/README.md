# Translate

## Goal

Provide quick translation from selected text, clipboard text, or typed query. The experience should feel like a Raycast command: summon, type or paste, translate, copy result, dismiss.

## Current Behavior

- `tr <text>` and `translate <text>` produce a translate action.
- The action calls the system Shortcuts runner with a shortcut named `Luma Translate`.
- The shortcut should accept text input and return translated text.
- The translated result is copied to the clipboard.

## MVP Behavior

- Query prefixes:
  - `tr <text>` translates text.
  - `translate <text>` translates text.
- Actions:
  - Copy translated result.
  - Replace selected text later via Accessibility.
  - Swap source/target language later.
- Default provider: system Shortcuts/Apple translation workflow.
- No HTTP provider abstraction until a concrete missing language requires it.

## Data

- Recent translation pairs may be cached locally.
- Cache must not store text classified as secret.

## UI Card

- Shows source language, target language, last translated snippet, and copy button.
- Edit button opens language preferences.

## Implementation Entry

- Source module: `Sources/LumaModules/Translate/TranslateModule.swift`
- Service boundary: `Sources/LumaServices/Translation/TranslationService.swift`
