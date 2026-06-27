# Translate

## Goal

Provide quick translation from selected text, clipboard text, or typed query. The experience should feel like a Raycast command: summon, type or paste, translate, copy result, dismiss.

## Current Behavior

- `tr <text>` and `translate <text>` produce a translate action.
- On macOS 26 and newer, the service attempts Apple Translation through `TranslationSession` after source language detection with `NLLanguageRecognizer`.
- On older macOS releases, or when the Apple Translation session is unavailable, the service falls back to a Shortcut named `Luma Translate`.
- The translated result is copied to the clipboard.

## MVP Behavior

- Query prefixes:
  - `tr <text>` translates text.
  - `translate <text>` translates text.
- Actions:
  - Copy translated result.
  - Replace selected text later via Accessibility.
  - Swap source/target language later.
- Default provider: Apple Translation on macOS 26+, with Shortcuts fallback.
- No HTTP provider abstraction until a concrete missing language requires it.

## Data

- Recent translation pairs may be cached locally.
- Cache must not store text classified as secret.

## Current UX

- `tr <text>` or `translate <text>` returns a translate result in the unified list.
- Return copies the translated text and can enter the same-panel Translate detail view where supported.
- Language chips and retry behavior live inside the panel, not on a dashboard card.

## Implementation Entry

- Source module: `Sources/LumaModules/Translate/TranslateModule.swift`
- Service boundary: `Sources/LumaServices/Translation/TranslationService.swift`
