# Command Recipes

Semantic command templates for Luma. A recipe name (for example `test`) maps to ordered
`program + args` steps chosen by the current project environment — not shell history and not
an autonomous agent.

## Query

| Query | Meaning |
| --- | --- |
| `cmd` | List recipes |
| `cmd test` | Search recipes matching `test` |
| `recipe test` | Alias for `cmd test` |
| `recipes` | Alias trigger |

TUI shortcuts while a recipe row is selected:

| Key | Action |
| --- | --- |
| Enter | Preview |
| `r` | Run |
| `c` | Copy commands |
| `f` | Favorite / unfavorite |

Run executes in the **current terminal** (TUI suspends, command output appears, then Luma resumes).

## CLI

```bash
luma query "cmd test" --json
luma cmd list [--json]
luma cmd show <recipe-id> [--json]
luma cmd run <recipe-id> [--confirmation] [--json]
luma cmd copy <recipe-id> [--json]
```

`run` inherits stdin/stdout/stderr. Non-safe recipes require `--confirmation`.

## Configuration

User recipes:

```text
~/Library/Application Support/LumaNext/command-recipes.toml
```

Tests must set `LUMA_NEXT_SUPPORT_DIR` (never read the real path above).

Metadata only (favorite, last used, use count, last result):

```text
~/Library/Application Support/LumaNext/command-recipes-meta.sqlite
```

### TOML shape

```toml
[[recipes]]
id = "test"
title = "Run project tests"
description = "Run the test command appropriate for the current project"
tags = ["test", "project"]
risk = "safe"
scope = "current_project"

[[recipes.variants]]
id = "rust"
description = "Rust project"
requires_files = ["Cargo.toml"]
requires_commands = ["cargo"]

[[recipes.variants.steps]]
id = "cargo-test"
label = "cargo test"
program = "cargo"
args = ["test", "--workspace", "--all-features"]
cwd = "current"
```

Merge rules:

1. Built-in recipes always exist.
2. Same `id` in user TOML replaces the built-in recipe.
3. `enabled = false` disables a recipe.
4. Duplicate ids in user TOML are reported and skipped.
5. TOML errors surface as module `unavailable` — Luma keeps running.
6. Unknown `risk` values are treated as `confirm`.

## Variant matching

From the current working directory:

- All `requires_files` must exist (regular files; symlinks rejected).
- All `requires_directories` must exist.
- All `requires_commands` must be found on `PATH`.
- The most specific matching variant wins (more requirements beats fewer).
- Ties keep recipe definition order.
- No match → subtitle `当前项目不适用`; Run is not offered.

`cwd` on each step is `current` or a relative path under the working directory.

## Risk

| Level | Typical use |
| --- | --- |
| `safe` | Status, tests, fmt check, logs (built-ins default) |
| `confirm` | Writes, installs, service starts |
| `destructive` | Deletes, resets, force operations |

Confirm/destructive recipes require explicit confirmation in TUI and `--confirmation` in CLI.

## Built-in recipes

**Git:** `git-status`, `git-diff`, `git-log`, `git-branch`, `git-worktree`

**Rust:** `rust-fmt-check`, `rust-check`, `rust-test`, `rust-clippy`

**Node:** `node-test`, `node-lint`, `node-build`

**Python:** `python-test`, `python-lint`

**Docker:** `docker-ps`, `docker-compose-ps`, `docker-compose-logs`

**Local:** `ports-list`, `git-root`, `show-env` (secrets redacted)

**Luma repo:** `luma-check`, `luma-test`, `luma-architecture-check` (`scope = luma_repository`)

**Generic:** `test` (Rust / Node / Python by project files)

## Safety

- Steps use `program` + `args` only — no `sh -c`.
- No new Terminal windows, AppleScript, or `open` for execution.
- `show-env` filters names/values that look like secrets.
- Command output is not persisted.

## Limits (v1)

- No shell pipes, redirects, or conditionals.
- No in-TUI recipe editor (edit TOML).
- No shell-history import, daemon, or cloud sync.
