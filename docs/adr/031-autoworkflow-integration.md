# ADR-031: Auto Workflow cc-loop Integration

**Status:** Accepted
**Date:** 2026-07-01

## Context

Auto Workflow integrates the external `cc-loop` CLI, a personal coding automation runner that orchestrates planner, implementer, and reviewer loops in isolated git worktrees. Luma needs to configure, start, stop, and observe these workflows from the launcher without pulling `cc-loop` internals into the app.

## Decision

Integrate Auto Workflow as a first-class, default-off Luma module. No changes are made to `cc-loop`; Luma consumes its CLI contract: `doctor`, `init`, `auto --detach`, `resume`, `status --json`, and `list --json`.

### Architecture

```
┌──────────────────────────────────────────────────┐
│ LumaApp                                          │
│  AppCoordinator → autoworkflowModule             │
│  AutoworkflowDetailView (NSView control panel)   │
│  AutoworkflowSettingsView (SwiftUI config)       │
├──────────────────────────────────────────────────┤
│ LumaModules                                      │
│  AutoworkflowModule (actor, LumaModule)          │
│  AutoworkflowModuleBundle (static manifest)     │
├──────────────────────────────────────────────────┤
│ LumaServices                                     │
│  AutoworkflowService (actor, Process wrapper)    │
│  AutoworkflowServiceProtocol (testable)          │
│  AutoworkflowModels (tasks, snapshots, config)   │
│  AutoworkflowConfigStore (UserDefaults)          │
├──────────────────────────────────────────────────┤
│ cc-loop (external CLI, no changes)               │
│  ~/.cc-loop/tasks/<id>/state.json               │
│  runner.pid, runner.log                          │
└──────────────────────────────────────────────────┘
```

### Key Design Choices

1. **Thin adapter, not deep integration.** `AutoworkflowService` wraps `Process` calls to `cc-loop`; it never imports Python modules.

2. **Module follows the existing bundle pattern.** `AutoworkflowModuleBundle` registers through `ModuleRegistry.allBundles`. The module has `.onDemand` warmup tier and is default-off, so it does not enter the startup hot path unless enabled.

3. **Config persistence stays separate.** `AutoworkflowConfigStore` uses `UserDefaults` keys with the `aw_*` prefix, avoiding new core `ConfigurationStore` surface area.

4. **Trigger keyword `aw` / `auto` / `workflow`.** Opens the control panel detail view. Sub-commands `aw status`, `aw list` surface task summaries inline.

5. **Detail view is the control surface.** Start/stop/status/log viewing lives in the Auto Workflow panel, not in the search results list. This keeps the hot path clean.

6. **Settings section added.** `Settings → Auto Workflow` shows availability status, configurable paths (source, state root), and default providers (planner/reviewer/implementer/model). CLI lookup augments the macOS GUI app `PATH` with common Homebrew and user-bin locations.

### State Machine

```
Idle → [Init] → Initialized → [Start] → Running → [Poll status]
                                                ↓ [Stop / Complete / Fail]
                                              Stopped / Done / Failed
```

The detail view polls `cc-loop status --json` every 2 seconds during active runs. Stop reads `runner.pid`, verifies the command line belongs to `cc-loop`/Auto Workflow, sends SIGTERM, and escalates to SIGKILL if needed.

### Main Touchpoints

- `Sources/LumaServices/Autoworkflow/` — config, process wrapper, JSON codec, models, protocol.
- `Sources/LumaModules/Autoworkflow/` — module and bundle registration.
- `Sources/LumaApp/Launcher/AutoworkflowDetailView.swift` — same-panel control surface.
- `Sources/LumaApp/Settings/SettingsSwiftUIView.swift` — Auto Workflow settings section.

## Consequences

- **Positive:** Users can configure and launch cc-loop workflows from Luma, observe live status and logs, and stop running workflows.
- **Positive:** No changes to cc-loop. The CLI contract is sufficient for full lifecycle management.
- **Positive:** Module is onDemand — zero hot-path impact. Detail view polls every 2s during active runs, but only when panel is open.
- **Neutral:** Requires `cc-loop` to be installed and reachable from the augmented app `PATH`.
- **Risk:** If cc-loop schema changes (new `status --json` fields), the decodable structs need updating. Mitigated by cc-loop's semantic versioning contract (patch=bug fix, minor=backward-compatible additions, major=schema bump).

## Verification

1. `swift build` ✅
2. `swift test` — 554 tests passing ✅
3. `swift test --filter AutoworkflowServiceTests` — process/JSON/config-path coverage ✅
4. `./scripts/build_app.sh` — app bundle builds and signs ✅
5. Manual acceptance: enable module → `aw` → configure → start → status/logs → stop/resume.
