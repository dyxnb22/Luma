# Quicklinks

Quicklinks opens URL templates from exact trigger words. Built-in examples include `gh <query>` for GitHub, `g <query>` for Google, and `swift <query>` for Apple Developer search. `ql` / `quicklinks` opens the same-panel management view.

Configuration lives at:

`~/Library/Application Support/Luma/quicklinks.json`

Each row defines `name`, `trigger`, `urlTemplate`, optional `openWith` bundle ID, and optional SF Symbol `icon`. Templates reuse `SnippetVariableExpander` variables: `{{query}}`, `{{clipboard}}`, `{{selection}}`, `{{project}}`, `{{project_path}}`, `{{uuid}}`, `{{timestamp}}`, and `{{date:fmt}}`.

v1 intentionally avoids fuzzy fallback, groups, drag sorting, icon picking, and edit actions in the action panel. Only the first token can match a trigger, so Quicklinks does not pollute normal app or note searches.
