# Ports

Personal listening-port switcher — not a Docker Desktop or process monitor clone.

## What it does

- Lists listening **TCP** ports (via `lsof` on macOS)
- Search / filter by port number, process name, command line, or local display name
- Preview: address, PID, process, command, favorite / last-seen metadata
- Primary action: **Kill (SIGTERM)** with confirmation
- Action picker: force kill (SIGKILL), copy port / PID / command, favorite, clear name
- Persist names and favorites under LumaNext `ports_meta.sqlite`

## Triggers

`port` · `ports` · `kill`

Examples:

| Query | Behavior |
| --- | --- |
| `port ` | All listening TCP ports |
| `port 3000` | Exact / partial port match |
| `port node` | Process / command match |
| `port fav` | Favorites only |
| `port name 3000 api` | Save local display name |
| `port unname 3000` | Clear name (confirm) |
| `port reload` | Refresh catalog |

## CLI

```bash
luma ports list --json
luma ports list 3000 --json
luma ports name 3000 api
luma ports favorite 3000
luma ports kill 3000 --yes
luma ports kill 3000 --force --yes
luma query "port node" --json
```

## Safety

- Never kills PID 0/1 or the Luma process itself
- Kill / force-kill require confirmation (`--yes` on CLI)
- No shell string concatenation — `lsof` / `kill` use argv arrays
- Missing `lsof` → `unavailable` row (no silent empty list)
- Permission failures → `permission_required` with local guidance
- Tests use `FakeProcessCatalog` only — never signal real processes

## Non-goals

- Docker / OrbStack container management
- Full Activity Monitor clone
- UDP / non-listening sockets
- Automatic kill without confirmation
- Background polling daemon
