# Luma Workbench Strategy

## Product Positioning

Luma is **not** a Notion, Things, or Alfred clone. It is a **macOS-native, command-first personal workbench** — a fast surface for capturing context, continuing work, converting between formats, and executing the next step without leaving the keyboard.

**Core value (evolved):** Luma helps you **resume the work around a project**, not just reopen an app or search a module.

- **Fast capture** — turn selection, clipboard, or app context into drafts and work items
- **Fast continue** — resume recent projects, drafts, project activities, and in-progress modules from Home
- **Fast convert** — clipboard → snippet, URL → quicklink, text → todo/note
- **Fast execute** — command triggers for targeted actions without global search fan-out
- **Project workspace** — lightweight project attribution, activity trail, and workspace detail
- **Local-first** — privacy-friendly, low-distraction, no cloud-sync dependency

Route C (Command-First Unified List) remains the active UI model. Workbench Core adds a semantic layer above modules without replacing `ModuleHost`.

## MVP User Scenarios

### 1. IDE project URL → project quicklink → Home continue

User works in a matched IDE project, copies a URL, and saves it as a project quicklink from Home or `save url`. Activity records project attribution. Next launcher open shows project CONTINUE rows (workspace, recent draft) without warming unrelated modules.

### 2. Selection → attach to project → todo/note/snippet

User selects text in the IDE and uses Home **Attach selection to project** or `attach sel`. Return executes capture; activity records source, project, and target module. Resume row or project workspace detail lets them continue editing.

### 3. Open Luma → current project recent activity → continue

User opens Luma with an active project context. Home shows **Continue project workspace**, recent project drafts from `WorkbenchActivityStore`, and attach clipboard/selection rows. `proj work` opens the project workspace detail with recent activity and quick capture actions.

### 4. Copy content → snippet / todo / note (baseline)

User copies text or a URL. Opening Luma shows contextual CREATE rows (when modules are pinned + enabled). User can also type `cap clip todo` or `cap clip note` — preview row only, Return executes. Draft lands in the target module detail or daily note; Resume row appears on next Home visit.

## Non-Goals (Current Phase)

- Full project management (Gantt, issues, teams, kanban)
- Bidirectional knowledge graph
- Cloud sync / multi-device workbench state
- Automatic full-disk file indexing
- Home as an activity feed or dashboard grid
- Multi-window workbench UI
- Complete knowledge-base editor

## Success Metrics

### Engineering

| Metric | Target / guardrail |
| --- | --- |
| Cold launch → first query | p95 ≤ 50 ms (panel + home) |
| Global search fan-out module count | ≤ hot-path tier only (`ModuleRegistry.globalSearchModuleIDs`) |
| Warm modules after 5 min idle | ≤ pinned + reserved + recently used on-demand |
| Capture path module warmups | 0 unrelated modules warmed |
| Activity query | Reads `workbench-activity.json` only; no module store scans |
| Command preview side effects | 0 captures/writes before Return |
| Activity JSON size | ≤ 50 entries; v1 envelope stable |
| Test suite | `swift test` green on every phase |

### Product

| Metric | Direction |
| --- | --- |
| Capture → draft steps | ≤ 2 (command or Home row → detail) |
| Continue last work steps | ≤ 1 (Home CONTINUE → Return) |
| Project attribution on capture | Present when `currentProject` exists |
| Project Home first screen | Shows workspace + recent draft when activity exists |
| Home first-screen actionable rows | 4–8, low noise (caps via `HomeSuggestionPolicy`) |
| Disabled module leakage | 0 targeted/capture/command/detail invocations |
| Project workspace open | Warms Projects (+ target module on execute only) |

## Architecture Boundary

```
WorkbenchContextBuilder → WorkbenchContext (current work-state snapshot)
WorkbenchActivitySnapshot → globalRecent + currentProjectRecent + currentProjectDrafts
WorkbenchActivityStore    → local workbench memory (v1 JSON envelope)
WorkbenchCapture          → draft builders (LumaModules) + resume/activity
CurrentProjectWorkspaceModelBuilder → pure detail view model (section order + gates)
ModuleHost                → lifecycle, warmup, teardown (unchanged)
QueryDispatcher           → global + targeted search (unchanged)
```

**Project workspace read model:** `WorkbenchContextBuilder` builds `WorkbenchActivitySnapshot` from the full activity store using `currentProject.matchedProjectPath ?? projectLabel`. Home (`ProjectActivityHomeContributor`), `proj recent`, and `CurrentProjectDetailView` all read `activitySnapshot.currentProjectRecent` / `currentProjectDrafts` — never filter `globalRecent` (top 8) by project.

See [ARCHITECTURE.md](ARCHITECTURE.md) and [specs/PERFORMANCE.md](specs/PERFORMANCE.md) for runtime details.
