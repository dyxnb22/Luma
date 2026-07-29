# `luma.ssh`

`luma.ssh` is a lightweight personal SSH workspace: it lists concrete `Host` aliases from your
OpenSSH config, embeds an SSH terminal inside Luma, and keeps **Luma-local** metadata (favorites,
display names, recent connections). It covers basic Termius-style connection management, but
does not provide multi-session tabs, a port-forward UI, sync, or a built-in file browser.

## What it reads

| Source | Role |
| --- | --- |
| `~/.ssh/config` | Host aliases (`Include` supported, depth 8) |
| `ssh -G <alias>` | Resolved hostname, user, port, identity file, ProxyJump, connect timeout |
| `~/Library/Application Support/LumaNext/ssh_meta.sqlite` | Favorites, display names, `last_connected_at`, `connection_count` |
| macOS Keychain service `com.luma.next.ssh-passwords` | Optional SSH passwords under private `ssh-password:<alias>` accounts |

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
Opening any targeted `/ssh …` query automatically re-reads the main config and `Include` files
and invalidates resolved-host cache entries, so HostName/User/Port edits do not require a restart.
`/ssh reload` remains an explicit manual refresh row.

Unprefixed `ssh` text is a global search; use `/ssh ` (with space) or Hub Enter on the SSH
module row.

## Actions

| Action | Effect |
| --- | --- |
| **Connect** (Enter) | Open **SSH Workspace** inside Luma (embedded child PTY). Records metadata on exit 0 |
| **Connect (compat mode)** | Legacy suspend TUI → `ssh <alias>` → resume |
| **Open SFTP** | Suspend TUI → `sftp <alias>` → resume (unchanged) |
| **Copy alias** | Writes the Host alias to the pasteboard |
| **Favorite** / **Unfavorite** | Updates `ssh_meta.sqlite` |
| **Delete local metadata** | Removes Luma row for alias (destructive, confirm) |
| **Reload** (`/ssh reload` row) | Clears alias / `ssh -G` caches |

### SSH Workspace

Embedded session keeps Luma chrome visible. Header shows alias · user@host:port · status.
On wide terminals the command shelf is visible on the right from the start while keyboard focus
stays in the remote terminal; `F6` focuses it, then closes it when pressed again. On narrower
terminals `F6` opens/closes the shelf. `Ctrl+Space` arms the terminal leader:
`Space` sends Ctrl+Space, `f` toggles fullscreen, `d` disconnects after a second confirmation
chord, `r` reconnects, and `q` leaves the workspace. Terminal `Esc`, Delete, PageUp/PageDown,
Alt keys, and F1–F12 are forwarded to the remote program. `Shift+PageUp/PageDown` browses the
local 2000-line scrollback; typing returns to live output.

The built-in shelf includes SSH/SFTP connection copy actions and grouped System, Files,
Services, Network, and Docker commands. Commands that need a service, container, path, host, or
URL open a parameter form before Copy/Insert. Empty optional parameters are omitted from the
rendered command.

Bracketed paste is forwarded as one remote paste operation when requested by the remote shell.
The real PTY cursor is projected into Luma so terminal input and IME candidate windows stay
anchored correctly. From the shelf, `Esc` returns to the terminal, `c` copies, `i` inserts
without Enter, and Enter previews a command or activates a native reconnect/disconnect action.
See [ADR-0008](adr/0008-ssh-workspace.md).

Preview shows resolved connection fields and metadata. Private key **contents** are never
shown — only the identity file path (sanitized).

If an embedded session fails, the last screen and status stay in the workspace (reconnect /
compat / leave); `Esc` then returns to `/ssh`. Compat-mode failures still pause for Enter before
restoring the TUI.

Entering `exit` in a healthy session returns to `/ssh` automatically after recording successful
session metadata.

When a password is saved for an alias, Connect and Open SFTP use OpenSSH `SSH_ASKPASS` to retrieve
that one value from macOS Keychain. The password never enters SSH metadata, search results,
action payloads, logs, argv, or the subprocess environment. Without a saved password, OpenSSH
keeps its normal interactive prompt.

## CLI

```bash
luma query "/ssh production" --json
luma ssh list --json
luma ssh connect production
luma ssh sftp production
luma ssh favorite production
luma ssh unfavorite production
luma ssh rename production "Prod server"
luma ssh password set production
luma ssh password status production
luma ssh password delete production
```

`connect` / `sftp` run in the foreground (no TUI suspend). On success they record connection
metadata the same way as the TUI path.

`password set` accepts a hidden interactive value, or stdin for scripting. Only the namespaced
Keychain account is passed through the connection action; an internal AskPass invocation reads
the value at authentication time. Saved SSH password accounts are private and do not appear in
the general `/sec` label list.

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
- Password sync or export (saved passwords remain in the local macOS Keychain)
- Non-macOS adapters (implementation uses `MacSshConfig` + OpenSSH on PATH)

## Tests

Unit and contract tests use `FakeSshConfigPort` and `LUMA_NEXT_*` temp dirs — they never read
the real `~/.ssh/config`. Blackbox covers `not_configured` and module registration.
