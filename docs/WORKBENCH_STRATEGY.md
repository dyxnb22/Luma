# Luma Workbench Strategy

## Product Positioning

Luma is **not** a Notion, Things, or Alfred clone. It is a **macOS-native, command-first personal workbench** — a fast surface for capturing context, continuing work, converting between formats, and executing the next step without leaving the keyboard.

**Core value:** Luma helps you **resume the work around a project**, not just reopen an app or search a module. Workbench entities (projects, drafts, links) are first-class objects with stable identity — not just an activity event stream.

- **Fast capture** — turn selection, clipboard, or app context into drafts and work items (≤ 5 seconds)
- **Fast continue** — resume projects, drafts, and modules via **`proj` / module triggers / detail** (not empty-query home rows)
- **Fast convert** — clipboard → snippet, URL → quicklink, text → todo/note
- **Fast execute** — command triggers for targeted actions without global search fan-out
- **Project workspace** — stable `ProjectIdentity`, activity trail, entity links, workspace detail
- **Local-first** — privacy-friendly, low-distraction, no cloud-sync dependency

Route C (Command-First Unified List) remains the active UI model. Workbench Core adds a semantic layer above modules without replacing `ModuleHost`.

## Core User Scenarios

### 1. Coding project workspace

User works in a matched IDE project. **`proj work` / `proj open`** opens workspace detail with quick capture, linked items, and actionable recent activity. Activity is keyed by `stableProjectID` (path hash) — same label, different paths never mix.

### 2. Daily capture inbox

User copies text or URLs throughout the day. `cap clip todo`, `proj capture`, or module search preview without side effects; Return executes once. Captures write activity + project links when a project context exists. Disabled modules never appear in command/detail surfaces gated by enablement.

### 3. Research / link / snippet collection

User saves URLs as project quicklinks, attaches clipboard/selection to a project snippet, and reviews linked items via **`proj links`** or project workspace detail. Linked items come from `workbench-links.json`, not global recent top 8.

## User Workflow Closure

**Capture → Attach/Organize → Continue → Review**

| Stage | Surfaces |
| --- | --- |
| Capture | `proj capture`, detail Quick capture, `cap clip/sel …`, module search |
| Attach/Organize | `attach clip/sel`, project capture → `WorkbenchLinkStore` |
| Continue | `proj resume`, `proj recent`, detail activity buttons, module triggers (`word`, `t`, …) |
| Review | `proj links`, project workspace detail Linked items section |

## MVP Completion Criteria

| Criterion | Target |
| --- | --- |
| Capture to draft | ≤ 2 steps (`cap` / `proj capture` / command → detail or inline) |
| Continue last work | ≤ 1 step (`proj resume` / module trigger → Return) |
| Project context accuracy | `stableProjectID` path-first; legacy label fallback |
| Disabled module leakage | 0 rows in command / detail / resume surfaces |
| Hot-path performance | No unrelated module warmup; activity + links JSON on demand (detail / `proj`) |

## Non-Goals (Current Phase)

- Full project management (Gantt, issues, teams, kanban)
- Bidirectional knowledge graph or cloud sync
- Home as activity feed grid
- Module store scans on Home/command path

## Success Metrics (Engineering)

| Metric | Target / guardrail |
| --- | --- |
| Cold launch → first query | p95 ≤ 50 ms |
| Global search fan-out | ≤ `ModuleRegistry.globalSearchModuleIDs` |
| Capture path module warmups | 0 unrelated modules |
| Activity + links read | `workbench-activity.json` + `workbench-links.json` only |
| Command preview side effects | 0 before Return |
| Activity schema | v2 envelope; v1/legacy auto-migrate |
| Test suite | `swift test` green |

## Architecture Boundary

```
ProjectIdentity (stableProjectID) → activity query + link index
WorkbenchActivityStore (v2)         → activity entries + entityRef
WorkbenchLinkStore                → project → entity links (+ lazy backfill)
WorkbenchLinkedEntityOpenPlanner  → entry/link → row action (single source)
WorkbenchActivityRowActions       → shared presentation + launcher Action encoding
WorkbenchWorkspaceRowActionCodec  → row action → Action / WorkbenchCommandOutcome
WorkbenchEntityResolver           → entry → WorkbenchEntityRef
WorkbenchContextBuilder           → activitySnapshot + linkSnapshot
CurrentProjectWorkspaceModelBuilder → detail sections (pure)
ModuleHost / QueryDispatcher        → unchanged hot path
```

### Activity row semantics (Beta)

`proj recent` preview rows and detail recent activity both use **`WorkbenchLinkedEntityOpenPlanner`** via `WorkbenchActivityRowActions.presentation(for:)`. Return encodes the planner result directly (`replaceQuery`, `openModuleDetail`, `resumeActivity`, or `showStatus` for recorded activity). Command preview remains side-effect free.

### Link index backfill

When `workbench-links.json` is empty, or the **current project** has no indexed links, `WorkbenchLinkStore.ensureLinksIndexed` derives links from any project activity where **`WorkbenchLinkIndexing.isLinkEligible`** (`projectIdentity` + resolvable `entityRef`) — matching live `recordLink` after capture. Includes `draftPrepared` project captures (todo/note/quicklink), not only `projectLinked`. Dedupe key: `stableProjectID + entityRef.kind + entityRef.entityID` (title/subtitle changes update in place). ≤ 100 cap. Triggered lazily from `WorkbenchContextBuilder` and detail activate.

### Diagnostics and empty states

- **`proj status`** — read-only snapshot row: `stableProjectID`, activity count, enabled link count; Return shows full status in hint bar.
- **`WorkbenchEmptyStateCopy`** — shared strings for no project, no links, no recent activity, disabled modules.

### Note capture (Beta scope)

Note capture runs immediately via `NotesAction`; activity records do **not** persist a reopenable note path. Rows with legacy `noteReference` payload open the **Notes module**, not a specific file path.

See [ARCHITECTURE.md](ARCHITECTURE.md) and [specs/PERFORMANCE.md](specs/PERFORMANCE.md).
