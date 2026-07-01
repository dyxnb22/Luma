# Auto Workflow

Default-off `cc-loop` control surface inside Luma. It lets the user configure, start, stop, resume, and observe local coding automation tasks without making `cc-loop` part of Luma's runtime.

## Commands

- `aw` / `auto` / `workflow` — open the same-panel Auto Workflow detail view
- `aw status` — show recent task status rows
- `aw list` — show recent task summary rows

## Scope

- Luma wraps the external `cc-loop` CLI only.
- The module is `onDemand` and default-off, so it does not participate in global search or startup warmup unless the user enables and targets it.
- Settings owns path/provider configuration under Settings → Auto Workflow.
- Task state stays in the configured `cc-loop` state root, normally `~/.cc-loop`.

## UI Behavior

- The detail view runs doctor → init → detached start for a new task.
- Active tasks poll `status --json` every 2 seconds while the detail view is open.
- Logs show a bounded tail of `runner.log`.
- Stop verifies the PID command line before sending signals.

## Privacy And Safety

- No network API is added by Luma.
- Luma does not parse or import `cc-loop` Python internals.
- Process execution uses `Process`, not shell strings.
- CLI lookup augments the macOS GUI app PATH with common Homebrew and user-bin locations.

## Source

- ADR: `docs/adr/031-autoworkflow-integration.md`
- Module: `Sources/LumaModules/Autoworkflow/`
- Service: `Sources/LumaServices/Autoworkflow/`
- Detail: `Sources/LumaApp/Launcher/AutoworkflowDetailView.swift`
