# Notes v0.1 — Cursor Prompt Sequence

Status: operational prompt list for ADR-008 / `NOTES_MARKDOWN_CURSOR_PLAN.md`
Date: 2026-06-22

## How To Use This File

1. Open a fresh Cursor Composer session in the Luma repo.
2. Paste **Master Prompt** once at the start of the session.
3. Then paste **Phase Prompts 0 through 10** one at a time, in order.
4. After each phase, run `swift build` (and `swift test` where the phase added tests) in your shell. If green, paste the next prompt. If not, paste the error back to Cursor and let it fix before moving on.
5. Do **not** paste two phase prompts at once. Cursor will skip steps and freelance.
6. If Cursor produces work that adds features outside ADR-008 (backlinks, frontmatter parsing, transclusion, Markdown preview, full-text search, AI, multi-vault, etc.), paste the **Course-Correction Prompt** at the bottom of this file.

The plan doc (`docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md`) is the source of truth for exact file paths, type signatures, and acceptance checks. Every phase prompt below tells Cursor to read its corresponding section in that doc first.

---

## Master Prompt (paste once at session start)

```
You are implementing Luma's Notes v0.1 module across multiple phases. Before you write any code, read these documents in full and treat them as binding:

1. .claude/CLAUDE.md
2. docs/adr/008-notes-markdown-manager.md
3. docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md
4. docs/specs/MODULE_CONTRACT.md
5. docs/specs/PERFORMANCE.md
6. docs/adr/007-dashboard-widget-single-window.md

Hard rules for the entire session:

- Swift 6 strict concurrency. Modules are actors.
- macOS 14+. AppKit for launcher UI. SwiftUI only for Settings/About.
- The module contract is binding: `handle` does zero disk I/O, returns within its declared queryTimeout, and must never throw. Failures become diagnostics.
- Performance contract is binding: warm keystroke p95 above 30 ms is a regression and blocks the change.
- Deletes use `FileManager.trashItem`. Never `unlink` or `FileManager.removeItem` against user files.
- Never log note paths, filenames, or note contents above `.debug`. Use `os_log` with `.private` markers for user-controlled strings.
- Do not add features that ADR-008 lists as non-goals, even if they look small. The non-goals are: Markdown rendering, Markdown editing, page embedding / transclusion / `![[Note]]` rendering, wiki-link backlinks, tags, frontmatter parsing, frontmatter generation, templates, full-text body search, AI of any kind, multi-vault, vault sync, vault import/export, version history, drag-to-move tree nodes, image gallery, image compression, image format conversion, paste interception, and inbox/orphan/recent counts on the dashboard card.
- Do not delete or rename any file or symbol that is not explicitly named in the phase prompt I send.
- Do not modify routing, the launcher panel shell, the feature card grid layout, or any module other than Notes unless a phase prompt names it.
- If a phase asks for tests, place them in the existing test target structure (`Tests/LumaServicesTests/` or `Tests/LumaModulesTests/`).
- After each phase prompt, your final message must be a short report containing: files created, files modified, files deleted, the result of `swift build`, the result of `swift test` if you ran it, and any deviations from the prompt with the reason. Then stop and wait for the next phase prompt. Do not begin the next phase on your own.

Acknowledge by replying with: "Ready. Awaiting Phase 0." Do not write any code yet.
```

---

## Phase 0 Prompt — Demolish NotesGraph Scaffolding

```
Execute Phase 0 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

Concretely:

1. Delete these files entirely:
   - Sources/LumaModules/Notes/NotesGraphIndexer.swift
   - Sources/LumaModules/Notes/NotesVaultStore.swift

2. Search the entire repo for references to the deleted symbols (`NotesGraphIndexer`, `NotesVaultStore`, `NotesGraph`, `NoteFile`). Remove any registration or wiring in Sources/LumaModules/BuiltInModules.swift. Do not touch other modules.

3. In Sources/LumaModules/Notes/NotesModule.swift, change `displayName: "Notes Graph"` to `displayName: "Notes"`. Do not change anything else in that file in this phase. It will fail to compile because of the deleted store; that is expected and will be fixed in Phase 3.

4. Features/NotesGraph/README.md is already a pointer to ADR-008; do not modify it.

Acceptance:

- `grep -r "NotesGraphIndexer\|NotesVaultStore\|NoteFile\|Notes Graph" Sources` returns empty.
- `swift build` fails only inside NotesModule.swift (because of the now-missing store). No other files emit errors.

Report and stop.
```

---

## Phase 1 Prompt — Real FSEventsService

```
Execute Phase 1 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

Sources/LumaServices/FileSystem/FSEventsService.swift is currently an empty actor. Implement it for real per the section "Phase 1: Real FSEventsService" of the plan doc.

Required surface (copy verbatim):

    public struct FSChangeEvent: Sendable, Hashable {
        public enum Kind: Sendable { case created, removed, renamed, modified, unknown }
        public let path: String
        public let kind: Kind
    }

    public actor FSEventsService: FileSystemClient {
        public init()
        public func watch(root: URL, debounceMillis: Int = 200) -> AsyncStream<[FSChangeEvent]>
        public func stop(root: URL)
    }

Implementation rules from the plan doc apply in full. Pay attention to:

- Use FSEventStream with `kFSEventStreamCreateFlagFileEvents` and `kFSEventStreamCreateFlagNoDefer`.
- Set FSEvents `latency = 0.0`; do the debounce in Swift inside the actor.
- Map FSEvents flags to `FSChangeEvent.Kind` exactly as the plan specifies.
- Only emit events for paths that are directories or end in `.md` (case-insensitive).
- Implement `stop(root:)` that invalidates the stream and releases it.

Then add tests at Tests/LumaServicesTests/FSEventsServiceTests.swift covering:

- Create, rename, delete on a tmp directory each surface within 1 s.
- Burst of 50 creates emits ≤ 3 batches.

Acceptance:

- `swift build` passes (NotesModule still failing from Phase 0 is acceptable — it will be fixed in Phase 3; this phase touches only LumaServices).
- `swift test --filter FSEventsServiceTests` passes.

Do not modify anything outside Sources/LumaServices/FileSystem/ and the new test file. Report and stop.
```

---

## Phase 2 Prompt — NotesRootConfig + NotesTreeIndex

```
Execute Phase 2 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

Create two new files:

1. Sources/LumaModules/Notes/NotesRootConfig.swift — value types and config store as specified in the plan doc section "2.1 NotesRootConfig". Persistence path is `~/Library/Application Support/Luma/notes.json`. Write atomically. Decode failure returns `.empty`, never throws to the caller.

2. Sources/LumaModules/Notes/NotesTreeIndex.swift — the in-memory tree per the plan doc section "2.2 NotesTreeIndex". Honor these constraints:
   - Folders sort before notes; within group, `localizedStandardCompare` ascending.
   - Include only `.md` files (case-insensitive) and directories.
   - Skip hidden files (`.skipsHiddenFiles`).
   - `search(prefix:)`: case-insensitive prefix on `name`.
   - `search(fuzzy:)`: subsequence match scored by gap penalty `length / (length + gaps)`, ties broken by shorter name first.
   - No method in this type performs file reads on behalf of `handle`. The only file-touching methods are `warmup` and `rebuild`.

Then add Tests/LumaModulesTests/NotesTreeIndexTests.swift covering:

- Empty root → nil snapshot.
- 100-file vault warms up under 200 ms in DEBUG.
- Folder ordering: folders precede notes; alphabetical within groups.
- Prefix search hits the expected files.
- Fuzzy search ranks tighter matches first.

Acceptance:

- `swift build` succeeds for everything in LumaModules/Notes/ except NotesModule.swift (still broken from Phase 0).
- `swift test --filter NotesTreeIndexTests` passes.

Do not touch NotesModule.swift, FeatureCatalog, BuiltInModules, or anything outside LumaModules/Notes/. Report and stop.
```

---

## Phase 3 Prompt — Rewrite NotesModule (memory-only handle)

```
Execute Phase 3 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

1. Create Sources/LumaModules/Notes/NotesTypora.swift exactly per section "3.1 NotesTypora helper". Probe `/Applications/Typora.app`, then `~/Applications/Typora.app`. Fall back to `NSWorkspace.shared.open(url)`. No user prompts, no telemetry, no logging of paths.

2. Rewrite Sources/LumaModules/Notes/NotesModule.swift per section "3.2 Rewrite NotesModule". Critical requirements:
   - `handle` does zero disk I/O. It only calls `index.search(fuzzy:)` and maps to ResultItem.
   - Drop the legacy behavior where the module required the user to type `notes` or `note` as a prefix. Empty query returns no results; non-empty query returns up to 10 fuzzy matches.
   - `warmup` loads config, sets the index root, runs warmup, and starts the FSEvents subscription in a long-lived `Task` stored on the actor. Cancel that task in `teardown`.
   - `perform` decodes the URL from the action payload and calls `NotesTypora.open` on the main actor. Do not implement panel-hide here; the host handles it.
   - Update the manifest's `queryTimeout` to `.milliseconds(40)` if it differs.

3. Wire registration:
   - Sources/LumaModules/BuiltInModules.swift: ensure NotesModule is instantiated once and included in the module list.
   - Sources/LumaModules/FeatureCatalog.swift: confirm the Notes feature card entry uses SF Symbol `note.text` and the Notes gradient defined in docs/strategy/DASHBOARD_WIDGET_STRATEGY.md. Add the entry if missing; do not touch other entries.

Acceptance:

- `swift build` passes cleanly across the whole project.
- A static review of `NotesModule.handle` shows no file system calls, no `FileManager` use, no `String(contentsOf:)`.
- All existing tests still pass (`swift test`).

Do not implement the detail view in this phase. Report and stop.
```

---

## Phase 4 Prompt — Detail View

```
Execute Phase 4 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

Create:

1. Sources/LumaApp/Launcher/NotesOutlineDataSource.swift — NSOutlineViewDataSource + Delegate over NotesNode per section "4.1 NotesOutlineDataSource". Row height 24, 8 pt indent per level, SF Symbol `folder` for folders and `doc.text` for notes, label 13 pt medium. Expose closures for `onActivate(NotesNode)` triggered by double-click on notes and Return on selection.

2. Sources/LumaApp/Launcher/NotesDetailView.swift — UI per section "4.2 NotesDetailView". Critical rules:
   - Top strip 28 pt: root path on the left (truncated middle, 11 pt secondary), gear icon on the right with `NSMenu` items `Change Root…`, `Reveal Root in Finder`, `Image Tools…` (the Image Tools menu item is wired in Phase 8; in this phase make it disabled).
   - Center: NSScrollView containing NSOutlineView.
   - Empty state (no root configured): centered `Set Notes Root…` button which opens an `NSOpenPanel` (directories only), then writes the choice to `NotesRootConfigStore` and triggers re-warmup.
   - DO NOT add a search/filter field in this view. Filter is Phase 7. Search bar at the top of the panel stays the canonical search per ADR-007.
   - Default expansion: root expanded only. Persist expansion state in NotesRootConfig.expandedFolders on every expand/collapse.

3. Register the detail view:
   - Sources/LumaApp/Launcher/ModuleDetailViews.swift: register NotesDetailView for ModuleIdentifier.notes in ModuleDetailRegistry. Do not touch other registrations.

Acceptance:

- `swift build` passes.
- Manual: launching the app, clicking the Notes card opens the detail view inside the 860×540 panel. Sidebar and search bar remain visible. Esc returns to grid; Esc on grid closes panel.
- Without a configured root, the empty-state button is visible; selecting a folder via the open panel writes notes.json and renders the tree.
- After closing and reopening the panel, previously expanded folders remain expanded.

Do not implement create/rename/delete in this phase. Report and stop.
```

---

## Phase 5 Prompt — Create Note + Create Folder

```
Execute Phase 5 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

1. Create Sources/LumaModules/Notes/NotesActions.swift with the surface from section "5.1 NotesActions" (createNote + createFolder only; rename and trash come in Phase 6). Validation rules:
   - Trim whitespace; reject empty.
   - Reject names containing `/`.
   - Auto-append `.md` to note names that lack the extension (case-insensitive).
   - Reject if the target path already exists.
   - After success, call `index.rebuild` with a synthetic FSChangeEvent so the tree updates immediately and does not wait for FSEvents.

2. Wire into Sources/LumaApp/Launcher/NotesDetailView.swift:
   - Right-click on root row or folder row opens NSMenu with `New Note`, `New Folder`, `Reveal in Finder`.
   - Each create item opens a single-line NSAlert with an NSTextField; on OK, call NotesActions, then expand the target folder and select the new node.
   - Show validation errors as NSAlert with a single OK button.

Acceptance:

- `swift build` passes.
- Manual: creating `Tree` produces `Tree.md`; creating `Tree.md` keeps that exact name. Creating a duplicate fails with an error alert and leaves the filesystem untouched. The created node is selected and visible without waiting for FSEvents.

Do not implement rename, delete, or note-row context menus in this phase. Report and stop.
```

---

## Phase 6 Prompt — Rename, Delete, Right-Click Menus

```
Execute Phase 6 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

1. Extend Sources/LumaModules/Notes/NotesActions.swift with `rename(_:to:)` and `trash(_:)` per section "6.1 Extend NotesActions". Rules:
   - Rename uses the same name validation as create. Notes auto-append `.md`. Destination must not exist.
   - Trash uses `FileManager.trashItem`. Never `unlink` or `removeItem`.
   - Trash on folders refuses if the folder contains anything other than `.DS_Store`.
   - Always rebuild the index after success.

2. Extend Sources/LumaApp/Launcher/NotesDetailView.swift:
   - Folder row context menu: `New Note`, `New Folder`, `Rename`, `Delete`, `Reveal in Finder`. On the root row, hide `Rename` and `Delete`.
   - Note row context menu: `Open in Typora`, `Rename`, `Delete`, `Reveal in Finder`, `Copy Path`.
   - Keyboard: `Return` opens note in Typora or toggles folder expansion. `Delete` opens the delete confirmation alert. `F2` opens the rename alert.
   - Delete confirmation NSAlert: default button `Cancel`, destructive button `Move to Trash`. Folder non-empty case: NSAlert with text `This folder is not empty. Deleting non-empty folders is not supported in this version.` and a single OK button.

Acceptance:

- `swift build` passes.
- Manual: rename `foo.md` to `bar` produces `bar.md`; the renamed node is selected. Trashing a note moves it to `~/.Trash`. Attempting to trash a non-empty folder shows the expected alert and the filesystem is untouched.

Do not implement the in-tree filter or image tools in this phase. Report and stop.
```

---

## Phase 7 Prompt — In-Tree Filter

```
Execute Phase 7 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

1. Add a 28 pt filter row in Sources/LumaApp/Launcher/NotesDetailView.swift directly above the outline. Single NSTextField, leading SF Symbol `magnifyingglass`, placeholder `Filter notes and folders…`. This is local-only; it does NOT call into LauncherViewModel and it does NOT change the panel's top search bar behavior.

2. Extend Sources/LumaApp/Launcher/NotesOutlineDataSource.swift per section "7.2 Filter behavior":
   - When filter is non-empty, project a filtered tree containing matching nodes and all their ancestors.
   - Highlight matched substring on the label using NSAttributedString with `.foregroundColor = .controlAccentColor`.
   - Auto-expand all ancestors of matches.
   - Return key in the filter field selects the first match; Down arrow moves through matches.
   - Clearing the filter restores the previously persisted expansion state from NotesRootConfig.

Acceptance:

- `swift build` passes.
- Manual: typing `tree` in the filter expands the ancestor folders of any `Tree.md` and highlights the match. Clearing the filter restores prior collapse state without flicker.

Do not implement image tools or wiki-link jumps in this phase. Report and stop.
```

---

## Phase 8 Prompt — Image Tools Panel (P1)

```
Execute Phase 8 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

1. Create Sources/LumaModules/Notes/NotesImageTools.swift with the surface in section "8.1 NotesImageTools". Implementation rules:
   - Image extensions: png, jpg, jpeg, gif, webp, svg, heic (lowercase compare).
   - Parse `.md` files line by line; match `![alt](path)` and `<img src="path">` only. No HTML attribute parsing beyond `src="..."`.
   - Resolve relative paths against the note's directory.
   - Skip files inside the `_assets` (or configured) folder when scanning for orphans.
   - `migrateToAssets(folderName:)`: move every referenced image into `<root>/<folderName>/`, rewrite all references to the new relative path. Must be idempotent (second run reports 0 moved). Never delete a file; only move.
   - `checkTyporaConfig`: read `~/Library/Preferences/abnerworks.Typora.plist` if present; return human-readable warnings for absolute or empty copy-image paths.

2. Create Sources/LumaApp/Launcher/NotesImageToolsPanel.swift per section "8.2 NotesImageToolsPanel". Modal sheet over the launcher panel via `beginSheet`. Four buttons: `Scan Orphans`, `Scan Broken Links`, `Migrate Images to _assets/`, `Check Typora Config`. Results list below: 12 pt monospaced text, scrollable. Migrate requires a second confirmation alert listing the file count before executing.

3. Enable the `Image Tools…` gear menu item added in Phase 4 and wire it to open this sheet. Make sure the sheet's modal session uses the Notes root from NotesRootConfig.

Acceptance:

- `swift build` passes.
- Manual: scan + migrate + scan again — second scan reports 0 moved and 0 orphans for the same set.
- Manual: a `.md` referencing a missing image shows up in `Scan Broken Links`.
- Migration is reversible via `git checkout` (no file deletions).

Do not implement wiki-link jumps or recent notes in this phase. Report and stop.
```

---

## Phase 9 Prompt — Wiki Link Jump + Recent Notes (P2)

```
Execute Phase 9 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

1. Extend Sources/LumaModules/Notes/NotesActions.swift with `relatedNotes(in note: URL) async -> [URL]`:
   - Parse the note's body for `[[Title]]` mentions.
   - Match by exact filename (case-insensitive on `Title.md`).
   - Return unique URLs, in order of first appearance.
   - This is FORWARD links only. Do NOT add a backlinks implementation or persist any link index.

2. Wire a `Cmd+L` menu item on note rows in Sources/LumaApp/Launcher/NotesDetailView.swift labeled `Open Linked Notes…`. Show a small NSPopover anchored to the row listing matches. Return opens the selection in Typora.

3. Track recent notes in Sources/LumaModules/Notes/NotesModule.swift:
   - On every successful `perform` that opens a note, push the path to the front of an in-memory recents list, dedupe, cap at 8.
   - Persist as `recent: [path]` inside notes.json. Add the field to NotesRootConfig if missing; backward-compatible decode.
   - When the launcher query is empty and the detail view is visible, render the recent list as a flat pseudo-group at the top of the outline labeled `Recent`. Default collapsed. Hidden when empty.

Acceptance:

- `swift build` passes.
- Manual: a note containing `[[Tree]] and [[DP]]` opens a popover with both matches; Return opens the selected one.
- Manual: opening eight different notes populates the Recent group; the group survives an app restart.

Do not surface recent counts on the dashboard card. That is an ADR-008 non-goal. Report and stop.
```

---

## Phase 10 Prompt — Docs and Verification

```
Execute Phase 10 of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md.

Update existing docs only. Do not create new docs.

1. docs/specs/PERFORMANCE.md — add to the Latency Targets table:
   | `NotesModule.handle` | ≤ 10 ms | ≤ 30 ms | 40 ms (manifest queryTimeout) |
   And to Hot Path Rules: a bullet stating that Notes reads happen only in `warmup` and on FSEvents callbacks; `handle` never touches disk.

2. docs/NON_GOALS.md — append a `Notes Module` subsection mirroring the Explicit Non-Goals list in ADR-008.

3. docs/MANUAL_QA_CHECKLIST.md — append a `Notes v0.1` block with the checks listed in section "10.3 Manual QA checklist" of the plan doc.

4. docs/ROADMAP.md — add a Notes v0.1 milestone entry referencing ADR-008 and the eleven phases.

Final verification:

- `swift build -c release` is warning-free across new files.
- `swift test` is fully green.
- `grep -r "Notes Graph\|NotesGraphIndexer\|NotesVaultStore" Sources docs` returns empty (the Features/NotesGraph/README.md pointer file is allowed to mention the historical name; nothing else).

Report and stop. After this phase the module is complete per ADR-008.
```

---

## Course-Correction Prompt (paste if Cursor freelances)

```
Stop. You added work that is outside the current phase scope or that ADR-008 lists as a non-goal. Revert those changes. Re-read docs/adr/008-notes-markdown-manager.md and the active phase section of docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md. Then resume the active phase strictly as specified. Do not introduce any feature not named in the phase prompt I sent.
```

---

## Why Not One Giant Prompt

A single prompt covering all eleven phases would:

- Skip `swift build` between phases, masking compile errors until many files are wrong at once.
- Drop scope guardrails over a long generation (especially the ADR-008 non-goals, which Cursor likes to "helpfully" include).
- Make rollback expensive — one bad assumption near the start contaminates every later file.
- Defeat human review at the phase boundaries that the plan doc was structured to create.

The eleven-phase split exists because each boundary is a verifiable checkpoint. Use it.
