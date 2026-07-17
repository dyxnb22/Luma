# `luma.ssh`

`luma.ssh` is a personal SSH launcher: it lists concrete `Host` aliases from your OpenSSH
config, runs `ssh` / `sftp` in the **current terminal**, and keeps **Luma-local** metadata
(favorites, display names, recent connections). It is **not** a Termius-style client — no
session manager, port-forward UI, or built-in file browser.

## What it reads

| Source | Role |
| --- | --- |
| `~/.ssh/config` | Host aliases (`Include` supported, depth 8) |
| `ssh -G <alias>` | Resolved hostname, user, port, identity file, ProxyJump, connect timeout |
| `~/Library/Application Support/LumaNext/ssh_meta.sqlite` | Favorites, display names, `last_connected_at`, `connection_count` |

Luma **does not** edit `~/.ssh/config`. Display names and favorites live only in
`ssh_meta.sqlite`.

Wildcard `Host` patterns (`*`, `?`, `%`, `!` prefixes) are ignored — only concrete alias
names appear in search.

Override the config path for tests or tooling with `SSH_CONFIG`.

## TUI queries

| Query | Effect |
| --- | --- |
| `/ssh ` | List all configured hosts (hint row shows common verbs) |
| `/ssh <needle>` | Fuzzy match alias, display name, hostname, or user |
| `/ssh fav` / `/ssh favorites` | Favorites only |
| `/ssh recent` | Hosts with a recorded connection |
| `/ssh reload` / `/ssh refresh` | Re-read config and clear `ssh -G` cache |
| `/ssh rename ALIAS NAME` | Set a local display name (Enter or action picker to save). Prefix is case-insensitive; `NAME` may contain spaces. |

Sorting: **favorite first** → **most recently connected** → relevance score → alias.

Unprefixed `ssh` text is a global search; use `/ssh ` (with space) or Hub Enter on the SSH
module row.

## Actions

| Action | Effect |
| --- | --- |
| **Connect** (Enter) | Suspend TUI → `ssh <alias>` in current terminal → resume; records metadata on exit 0 |
| **Open SFTP** | Same flow with `sftp <alias>` |
| **Copy alias** | Writes the Host alias to the pasteboard |
| **Favorite** / **Unfavorite** | Updates `ssh_meta.sqlite` |
| **Delete local metadata** | Removes Luma row for alias (destructive, confirm) |
| **Reload** (`/ssh reload` row) | Clears alias / `ssh -G` caches |

Preview shows resolved connection fields and metadata. Private key **contents** are never
shown — only the identity file path (sanitized).

## CLI

```bash
luma query "/ssh production" --json
luma ssh list --json
luma ssh connect production
luma ssh sftp production
luma ssh favorite production
luma ssh unfavorite production
luma ssh rename production "Prod server"
```

`connect` / `sftp` run in the foreground (no TUI suspend). On success they record connection
metadata the same way as the TUI path.

Favorites and rename can also be driven through the engine:

```bash
luma action run --query "/ssh production" --action-id favorite
luma action run --query "/ssh rename prod Production" --action-id rename
```

## Status rows

| Kind | When |
| --- | --- |
| `not_configured` | `~/.ssh/config` missing |
| `unavailable` | Config parse/Include failure, `ssh` / `sftp` binary missing, or `ssh_meta.sqlite` open/read failure (hosts may still list; metadata actions fail until fixed) |
| `status` | Empty favorites/recent, no matches, or usage hints |

No centralized `doctor` — remediation text is on the row itself.

## Connection metadata

After a **successful** interactive session (`exit 0`), the engine records
`last_connected_at` (RFC 3339) and increments `connection_count`. Failed exits are not
recorded.

If `ssh_meta.sqlite` cannot be opened, the module still lists hosts; favorites and recent
filters simply have no persistence until the store is available.

## Out of scope

- Editing or generating `~/.ssh/config`
- Multiplexing, jump-host UI, or focusing an existing SSH window
- Tags, groups, or sync across machines (metadata is local to LumaNext)
- Non-macOS adapters (implementation uses `MacSshConfig` + OpenSSH on PATH)

## Tests

Unit and contract tests use `FakeSshConfigPort` and `LUMA_NEXT_*` temp dirs — they never read
the real `~/.ssh/config`. Blackbox covers `not_configured` and module registration.
