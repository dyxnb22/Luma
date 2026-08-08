# Modules

Luma is a small personal workbench. The optional native host is not a module: it only owns the
window, PTY, global activation, and lifecycle. `rust/bins/luma/src/compose.rs` is the sole module
registration root.

Interactive commands require a leading `/`; unprefixed input is always global search. Prefer an
honest `unavailable`, `permission_required`, or `not_configured` result over a silent empty state.

## Shell

| Area | Status | Notes |
| --- | --- | --- |
| Config | Available | `luma config get/set` and `/settings` cover project imports/root, Records root, Clipboard retention, Hub window count, theme, and module enable flags. Stale `enabled_modules` keys are ignored and left untouched. |
| Command discovery | Available | `Ctrl-/`, `/commands [filter]`, `/help`, and slash completion are generated from enabled module `CommandSpec` metadata. |
| Keyboard navigation | Available | Arrow keys move by row; `⌥↑`/`⌥↓` and `/scroll up|down` page the focused surface. `fn↑`/`fn↓` is a compact-Mac compatibility alias. |
| Doctor / diagnostics | Removed | Permission and availability states stay local to the owning module. |

## Active modules

| Module | Triggers | Purpose | Default |
| --- | --- | --- | --- |
| Apps | `/app` / `/apps` | Fuzzy app search with launch, reveal, and copy-path actions. | on |
| Windows | `/win` / `/window` / `/windows` | List/search visible windows and focus them when Accessibility is available; Hub supports window shortcuts `1`–`9`. | on |
| Projects | `/p` / `/proj` / `/project` | Manually imported projects and a project workbench that links Git, Runtime, Recipes, files, Finder, editor, and terminal actions. | on |
| Git | `/git` | Local-only imported-project dashboard: status, stage/unstage, diff, branches, log, commit, and confirmed tracked-file discard. No remote operations or `git clean`. | on |
| Runtime | `/run` / `/ports` | On-demand local TCP listeners with project association and confirmed, identity-rechecked same-user SIGTERM. | on |
| Command Recipes | `/cmd` / `/recipe` / `/recipes` | Explicit `program + args` recipes from built-ins and local TOML, including project-aware variants. | on |
| Clipboard | `/clip` / `/cb` | Local clipboard history, pin/unpin, clear, and session pause/resume. Concealed/transient password-manager types are skipped. | on |
| Wordbook | `/wb` / `/wordbook` / `/words` | Vocabulary lists, due-first review, grading/mastery, import, and backup. | on |
| Records | `/rec` / `/record` | SQLite-backed personal media records with search/browse, rating, notes, import, removal, and backup. | on |
| Fake | — | CLI/TUI test fixture only. | off |

The active production inventory is therefore **9 modules**. `Fake` is compiled only as a disabled
test/demo registration and is not part of the product surface.

## Why this boundary

The retained modules cover the workbench's recurring keyboard-first jobs: switching context,
opening projects, inspecting local development state, running known commands, and maintaining the
three personal datasets still owned by Luma. Features already handled well by macOS or focused
third-party tools are not duplicated.

The following modules were removed in the 2026-08 simplification:

- Calculator, Downloads Inbox, Packages, Apple Shortcuts Bridge, and Shell Recall;
- Renewals, Database Portals, Screen OCR, Proxy, Quicklinks, Snippets, and Timers;
- Secrets (use macOS Passwords/Keychain or another password manager instead);
- SSH, removed immediately before this simplification.

Removal includes composition, manifests, commands, ports/adapters, storage implementations,
settings, TUI affordances, and module tests. Luma deliberately does **not** delete old user files or
Keychain entries; retired SQLite databases, proxy files, and legacy secret/SSH entries may be
removed manually after the user decides they are no longer needed.

## Search, Recall, and Hub

- Global search contributors are Apps, Windows, Projects, Command Recipes, Clipboard, and Git.
  Records and Wordbook remain targeted-only; Runtime is targeted from `/run` and surfaced from a
  project workbench when associated.
- A successful natural primary action records bounded metadata in `recall.sqlite`. Clipboard bodies
  and submitted search text never enter Recall.
- The empty Hub shows visible Windows, up to three revalidated Recall objects, then active Modules.
  Continue rows never receive window digit shortcuts.
- Projects do not copy Git, Runtime, Recipe, or Recall data. `/proj show NAME|PATH` reads those
  sources on demand and links to their owning surfaces.

## Persistence

Current data under `~/Library/Application Support/LumaNext/` consists of settings plus the active
Recall, Clipboard, Wordbook, Records, Command Recipes, and migration-ledger stores. Backups created
by Wordbook and Records live under `LumaNext/backups/`. Logs remain under
`~/Library/Logs/LumaNext/` with bounded rotation.

Tests must use temporary roots through `LUMA_NEXT_SUPPORT_DIR` / `LUMA_NEXT_LOGS_DIR`; they must not
steal focus or mutate the real clipboard.

## Product rules

- The Rust CLI/TUI owns all workbench UI and module behavior; the Swift host owns only the window,
  PTY, activation, and lifecycle.
- Platform calls stay behind application ports, and `compose.rs` wires adapters to modules.
- Destructive or otherwise risky actions require confirmation and identity revalidation where
  applicable.
- Project removal edits settings only and never deletes a directory.
- Imported Records source Markdown stays read-only; `records.sqlite` is the post-import source of
  truth.
- There is no centralized doctor, diagnostics export, probe-port workflow, AI chat, agent daemon,
  or autonomous task loop.
