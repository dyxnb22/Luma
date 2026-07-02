# Auto Workflow cc-loop Evolution Notes

This document records Luma-side expectations for future cc-loop evolution. The
execution roadmap remains owned by cc-loop; Luma should continue to consume only
the external CLI and JSON contract.

## Integration Principle

Luma stays a thin adapter around cc-loop:

- Do not import cc-loop Python modules.
- Do not parse cc-loop internal artifacts as a primary contract.
- Treat `status --json`, `list --json`, `graph --json`, and future stable JSON
  commands as the source of truth.
- Prefer new cc-loop CLI commands over duplicating process control in Luma.

## Expected cc-loop Contract Additions

Future cc-loop versions should expose these capabilities before Luma relies on
them:

- `stop --task-id ID --json` for graceful runner termination.
- `cancel --task-id ID --json` for terminal task cancellation.
- `cleanup --task-id ID --json` for artifact/worktree cleanup.
- `report --task-id ID --json` for completed or failed task summaries.
- Heartbeat fields in `status --json`, including runner liveness and stale
  runner detection.
- Additive graph history fields or a separate `graph --history --json` command
  when dynamic replanning lands.

## Luma Upgrade Sequence

1. Replace direct PID termination with `cc-loop stop --json` once available.
2. Display heartbeat and runner state from `status --json`.
3. Add a report view backed by `report --json`.
4. Show task graph progress and graph history when cc-loop exposes stable fields.
5. Add provider/profile UI only after cc-loop supports per-node routing.
6. Add parallel node UI only after cc-loop exposes stable concurrent graph state.

## Non-Goals

- Luma does not become a workflow engine.
- Luma does not own cc-loop state transitions.
- Luma does not implement planner, reviewer, or implementer logic.
- Luma does not add enterprise governance; that belongs in a separate platform
  layer if needed.
