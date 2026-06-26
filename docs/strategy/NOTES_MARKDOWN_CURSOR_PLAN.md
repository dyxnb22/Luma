# Notes Markdown Manager — Cursor Implementation Plan

Status: **Historical implementation guide** for ADR-008. Notes functionality shipped, and this plan was written against the older Route B launcher shell. Use current active ADRs and code as the source of truth for launcher behavior.
Date: 2026-06-22
Companion: `docs/adr/008-notes-markdown-manager.md`

This plan delivers Notes v0.1 — a Markdown file index and Typora launcher — in eleven phases. Each phase is independently buildable, independently verifiable, and meant to be handed to Cursor Composer one prompt at a time.

## Cursor Rules

- Hand Cursor one phase, or one sub-phase, per prompt.
- Do not let Cursor reinterpret product scope. The non-goals in ADR-008 are not suggestions.
- Include exact file paths, exact constants, and exact acceptance checks in every prompt.
- If Cursor adds backlinks, frontmatter, tags, transclusion, Markdown rendering, multi-vault, or "smart" anything, stop and remind it of ADR-008.
- Run `swift build` after every sub-phase. Run the existing test suite after every full phase.
- New Swift files use Swift 6 strict concurrency. Modules are `actor` types unless stateless.
- Never call `unlink` or `FileManager.removeItem` for user files; always `FileManager.trashItem`.
- Never log note paths, filenames, or note contents at info level. Use `os_log` `.debug` with privacy markers.

## Architecture Snapshot

```
LumaServices/FileSystem/
  FSEventsService.swift           (stub today — real implementation in Phase 1)
  FileSystemClient.swift          (protocol, existing)

LumaModules/Notes/
  NotesModule.swift               (rewritten in Phase 3)
  NotesRootConfig.swift           (new, Phase 2)
  NotesTreeIndex.swift            (new, Phase 2)
  NotesActions.swift              (new, Phase 5)
  NotesImageTools.swift           (new, Phase 8)
  NotesTypora.swift               (new, Phase 3)

LumaApp/Launcher/
  NotesDetailView.swift           (new, Phase 4)
  NotesOutlineDataSource.swift    (new, Phase 4)
  NotesImageToolsPanel.swift      (new, Phase 8)
  ModuleDetailViews.swift         (registration update, Phase 4)

Deleted in Phase 0:
  LumaModules/Notes/NotesGraphIndexer.swift
  LumaModules/Notes/NotesVaultStore.swift
  Features/NotesGraph/README.md   (or marked superseded)
```

Persistence:

```
~/Library/Application Support/Luma/notes.json
```

Schema:

```json
{
  "root": "/Users/<name>/Documents/Notes",
  "expandedFolders": ["/Users/<name>/Documents/Notes/Projects"]
}
```

## Phase 0: Demolish NotesGraph Scaffolding

### 0.1 Delete dead files

Files:

- `Sources/LumaModules/Notes/NotesGraphIndexer.swift` — delete.
- `Sources/LumaModules/Notes/NotesVaultStore.swift` — delete.

Tasks:

- Remove both files.
- Remove any references from `Sources/LumaModules/BuiltInModules.swift` if present.

Acceptance:

- `swift build` fails on `NotesModule.swift` only (expected; fixed in Phase 3).
- No file in the repo references `NotesGraphIndexer`, `NotesVaultStore`, `NotesGraph`, or `NoteFile`.

### 0.2 Mark legacy README superseded

File:

- `Features/NotesGraph/README.md`

Task:

- Replace the file's content with a one-line pointer:
  ```
  # Notes Graph (superseded)
  This scope was retired by ADR-008. See `docs/adr/008-notes-markdown-manager.md` and `docs/strategy/NOTES_MARKDOWN_CURSOR_PLAN.md`.
  ```

Acceptance:

- File contains only the pointer text.

### 0.3 Rename module display

File:

- `Sources/LumaModules/Notes/NotesModule.swift`

Task:

- Change `displayName: "Notes Graph"` to `displayName: "Notes"`.
- Leave the rest of the file intact for now; Phase 3 rewrites it.

Acceptance:

- `grep -r "Notes Graph" Sources` returns nothing.

## Phase 1: Real FSEventsService

The current `FSEventsService.swift` is an empty actor. Notes cannot meet its contract without it.

### 1.1 Define the API

File:

- `Sources/LumaServices/FileSystem/FSEventsService.swift`

Required surface:

```swift
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
```

Tasks:

- Wrap `FSEventStreamCreate` with `kFSEventStreamCreateFlagFileEvents` and `kFSEventStreamCreateFlagNoDefer`.
- Use `latency = 0.0` at the FSEvents level; do the debounce in Swift inside the actor (`Task.sleep` after the first event, collect all in the window, emit one batch).
- Stop the stream and `FSEventStreamRelease` on `stop(root:)` and on actor deinit equivalent (since actors lack `deinit` for non-final resources, expose `stop` and call it explicitly).
- Map flags to `Kind`:
  - `kFSEventStreamEventFlagItemCreated` → `.created`
  - `kFSEventStreamEventFlagItemRemoved` → `.removed`
  - `kFSEventStreamEventFlagItemRenamed` → `.renamed`
  - `kFSEventStreamEventFlagItemModified` → `.modified`
  - otherwise `.unknown`
- Only emit events for paths whose extension is empty (directories) or `.md`.

### 1.2 Test the service

File:

- `Tests/LumaServicesTests/FSEventsServiceTests.swift`

Tasks:

- Create temporary directory.
- Start watching, create / rename / delete a `.md` file, assert events arrive within 1 second.
- Burst test: create 50 files in a loop; assert events come in ≤ 3 batches (debounce works).

Acceptance:

- `swift test --filter FSEventsServiceTests` passes.
- The actor releases the stream on stop (no FSEvents thread alive after `stop`; verify by checking that `Thread.callStackSymbols` does not contain `FSEvents` after `stop`).

## Phase 2: Tree Index and Root Config

### 2.1 NotesRootConfig

File:

- `Sources/LumaModules/Notes/NotesRootConfig.swift`

Required surface:

```swift
public struct NotesRootConfig: Codable, Sendable, Equatable {
    public var root: URL?
    public var expandedFolders: Set<String>
    public static let empty = NotesRootConfig(root: nil, expandedFolders: [])
}

public actor NotesRootConfigStore {
    public init(fileURL: URL = NotesRootConfigStore.defaultFileURL)
    public func load() -> NotesRootConfig
    public func save(_ config: NotesRootConfig) throws
    public static var defaultFileURL: URL { /* ~/Library/Application Support/Luma/notes.json */ }
}
```

Tasks:

- Atomic write via `Data.write(to:options:.atomic)`.
- Create parent directory if missing.
- Treat decode failure as `.empty` (do not throw).

### 2.2 NotesTreeIndex

File:

- `Sources/LumaModules/Notes/NotesTreeIndex.swift`

Required surface:

```swift
public struct NotesNode: Sendable, Hashable {
    public enum Kind: Sendable { case folder, note }
    public let path: String
    public let name: String
    public let kind: Kind
    public let children: [NotesNode]   // empty for notes
}

public actor NotesTreeIndex {
    public init()
    public func setRoot(_ root: URL?) async
    public func warmup() async                          // full scan, ≤ 1s soft budget
    public func rebuild(after events: [FSChangeEvent]) async
    public func snapshot() async -> NotesNode?          // root node or nil
    public func search(prefix query: String, limit: Int = 20) async -> [NotesNode]
    public func search(fuzzy query: String, limit: Int = 20) async -> [NotesNode]
}
```

Rules:

- Walk via `FileManager.enumerator` with `.skipsHiddenFiles`.
- Include only `.md` files (case-insensitive extension check) and directories.
- Sort siblings: folders before notes; within group, `localizedStandardCompare` ascending.
- `search(prefix:)` matches case-insensitive prefix on `name`; `search(fuzzy:)` does subsequence match scored by gap penalty (Subsequence Score = length / (length + gaps)).
- `handle` callers must never trigger I/O; all reads happen in `warmup` and `rebuild`.

### 2.3 Tests

File:

- `Tests/LumaModulesTests/NotesTreeIndexTests.swift`

Cover:

- Empty root returns nil snapshot.
- 100-file vault warms up under 200 ms in DEBUG.
- Folders sort before notes.
- Prefix search hits expected files.
- Fuzzy search ranks tighter matches higher.

Acceptance:

- All tests pass.
- `swift build -c release` reports no warnings in the new files.

## Phase 3: Rewrite NotesModule (Memory-Only handle)

### 3.1 NotesTypora helper

File:

- `Sources/LumaModules/Notes/NotesTypora.swift`

Required surface:

```swift
public enum NotesTypora {
    public static func open(_ url: URL)   // Typora if installed, NSWorkspace.open otherwise
}
```

Tasks:

- Probe `/Applications/Typora.app` then `~/Applications/Typora.app`.
- If found, use `NSWorkspace.shared.open([url], withApplicationAt:configuration:)`.
- Otherwise call `NSWorkspace.shared.open(url)`.
- No user-visible prompts. No telemetry.

### 3.2 Rewrite NotesModule

File:

- `Sources/LumaModules/Notes/NotesModule.swift`

Required surface:

```swift
public actor NotesModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .notes,
        displayName: "Notes",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(40)
    )

    public init()
    public init(index: NotesTreeIndex, config: NotesRootConfigStore)

    public func warmup() async
    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult
    public func perform(_ action: Action, context: ActionContext) async throws
    public func teardown() async
}
```

`handle` rules:

- Read query → call `index.search(fuzzy:)` → map top N (≤ 10) to `ResultItem`.
- Result subtitle: parent folder path relative to the root.
- Primary action: `kind: .custom(payload: pathData, handler: .notes)`.
- No `notes` / `note` keyword prefix gating any longer. Empty query returns no results.
- Zero disk I/O. Zero file reads.

`perform` rules:

- Decode payload back to `URL`.
- Call `NotesTypora.open(url)` on the main actor (`await MainActor.run { ... }`).
- Hide panel via existing action routing path; do not implement panel hiding here.

`warmup` rules:

- Load config from `NotesRootConfigStore`.
- If `root` is nil, do nothing (no error).
- Otherwise call `index.setRoot` + `index.warmup`, then start FSEvents subscription via a long-lived `Task`. Cancel on `teardown`.

### 3.3 Wire into FeatureCatalog / BuiltInModules

Files:

- `Sources/LumaModules/BuiltInModules.swift`
- `Sources/LumaModules/FeatureCatalog.swift`

Tasks:

- Construct `NotesModule()` exactly once and pass through the existing module list.
- Confirm the Notes feature card in `FeatureCatalog` uses `"note.text"` symbol and Notes gradient (per `DASHBOARD_WIDGET_STRATEGY.md`).

Acceptance:

- `swift build` passes.
- Launching the app with a configured root: typing `tree` in the top search bar surfaces `Tree.md` within one frame.
- With no configured root: typing in the launcher returns no Notes hits and no errors are logged.
- Instruments shows `NotesModule.handle` p95 ≤ 30 ms warm on a 1000-file vault.

## Phase 4: Detail View

### 4.1 NotesOutlineDataSource

File:

- `Sources/LumaApp/Launcher/NotesOutlineDataSource.swift`

Tasks:

- Implement `NSOutlineViewDataSource` and `NSOutlineViewDelegate` against `NotesNode`.
- Provide a single cell: 24 pt row height, SF Symbol `folder` or `doc.text`, 13 pt medium label, 8 pt indent per level.
- Selection style: `.regular`.
- Notify a closure on double-click of a note row and on Return key on a selection.

### 4.2 NotesDetailView

File:

- `Sources/LumaApp/Launcher/NotesDetailView.swift`

Required UI:

- Top strip (28 pt): root path on the left (`.secondaryLabelColor`, 11 pt regular, truncated middle), gear icon on the right that opens an `NSMenu` with `Change Root…`, `Reveal Root in Finder`, `Image Tools…`.
- Center: `NSScrollView` containing an `NSOutlineView`.
- Empty state when no root: centered `Set Notes Root…` button.

Rules:

- No second search field in this view. Search lives in the panel top bar; results overlay the grid per ADR-007.
- Default expansion: only the root node is expanded on first show.
- Persisted expansion state via `NotesRootConfig.expandedFolders` updated on `outlineViewItemDidExpand` / `…DidCollapse`.

### 4.3 Register detail view

File:

- `Sources/LumaApp/Launcher/ModuleDetailViews.swift`

Tasks:

- Register `NotesDetailView` for `ModuleIdentifier.notes` in `ModuleDetailRegistry`.

Acceptance:

- Click Notes card → detail opens in the 860×540 panel, sidebar and search bar still visible.
- Esc returns to feature grid; Esc on grid closes panel (per ADR-007).
- Reopening the panel restores previously expanded folders.

## Phase 5: Create Note and Create Folder

### 5.1 NotesActions

File:

- `Sources/LumaModules/Notes/NotesActions.swift`

Required surface:

```swift
public enum NotesActionError: Error, Sendable {
    case emptyName
    case nameContainsSlash
    case alreadyExists
    case rootMissing
}

public actor NotesActions {
    public init(index: NotesTreeIndex)
    public func createNote(name: String, inFolder folder: URL) async throws -> URL
    public func createFolder(name: String, inFolder folder: URL) async throws -> URL
}
```

Rules:

- Trim whitespace. Reject empty.
- Reject names containing `/`.
- Auto-append `.md` to note names that lack the extension (case-insensitive).
- Reject if the target path already exists (case-insensitive on macOS default APFS).
- Run `index.rebuild` with a synthetic `FSChangeEvent` after success so the tree updates without waiting for FSEvents.

### 5.2 Detail view wiring

File:

- `Sources/LumaApp/Launcher/NotesDetailView.swift`

Tasks:

- Right-click on the outline opens an `NSMenu`:
  - On root or folder rows: `New Note`, `New Folder`, `Reveal in Finder`.
  - On note rows: covered in Phase 6.
- "New Note" / "New Folder" use a single-line `NSAlert` with an `NSTextField` for the name. No multi-step wizard.
- On success, expand the target folder and select the new node.

Acceptance:

- Creating `Tree` produces `Tree.md`; creating `Tree.md` keeps that exact name.
- Same-name creation shows the alert error and leaves the filesystem unchanged.
- Created note appears in the tree immediately (not after FSEvents debounce).

## Phase 6: Rename, Delete, Right-Click

### 6.1 Extend NotesActions

File:

- `Sources/LumaModules/Notes/NotesActions.swift`

Add:

```swift
public enum NotesDeleteError: Error, Sendable {
    case folderNotEmpty
    case rootMissing
}

extension NotesActions {
    public func rename(_ url: URL, to newName: String) async throws -> URL
    public func trash(_ url: URL) async throws            // notes and empty folders only
}
```

Rules:

- Rename: same validation as create; auto-append `.md` for notes; refuse if destination exists.
- Trash: use `FileManager.trashItem`. For folders, refuse if non-empty (count children excluding `.DS_Store`).
- Update index after both operations.

### 6.2 Right-click menu and keyboard

File:

- `Sources/LumaApp/Launcher/NotesDetailView.swift`

Tasks:

- Folder row context menu: `New Note`, `New Folder`, `Rename`, `Delete`, `Reveal in Finder`.
- Note row context menu: `Open in Typora`, `Rename`, `Delete`, `Reveal in Finder`, `Copy Path`.
- Root row is a folder but cannot be renamed or deleted; hide those items when the row is the root.
- Keyboard: `Return` opens (note) or expands (folder). `Delete` opens the delete confirmation. `F2` opens rename.
- Delete confirmation: `NSAlert` with default button `Cancel`, destructive button `Move to Trash`. Folder non-empty case: alert text `This folder is not empty. Deleting non-empty folders is not supported in this version.` with single `OK` button.

Acceptance:

- Rename a note from `foo` to `bar`: file becomes `bar.md`, tree updates, selection follows the renamed node.
- Trash a note: file ends up in `~/.Trash`, tree updates within one frame.
- Attempt to trash a non-empty folder: alert appears, filesystem unchanged.

## Phase 7: In-Tree Filter

### 7.1 Filter input

File:

- `Sources/LumaApp/Launcher/NotesDetailView.swift`

Tasks:

- Add a 28 pt filter row immediately above the outline view. Single `NSTextField` with magnifying-glass leading icon, placeholder `Filter notes and folders…`.
- This filter is local to the detail view; it does **not** dispatch through the launcher search bar.

### 7.2 Filter behavior

File:

- `Sources/LumaApp/Launcher/NotesOutlineDataSource.swift`

Tasks:

- When filter is non-empty, project a filtered tree containing matching nodes and their ancestors.
- Highlight matched substring on the row label (use `NSAttributedString` with `.foregroundColor = .controlAccentColor`).
- Expand all ancestors of matches.
- Pressing Return selects the first match; pressing Down moves through matches.
- Clearing the filter restores the previously persisted expansion state.

Acceptance:

- Typing `tree` in the filter expands `Algorithms/`, highlights `Tree.md`, scrolls it into view.
- Clearing restores prior collapse state without lag.

## Phase 8: Image Tools (P1)

This phase implements ADR-008's four image utilities. They are one-shot commands inside an internal panel — no daily-use UI.

### 8.1 NotesImageTools

File:

- `Sources/LumaModules/Notes/NotesImageTools.swift`

Required surface:

```swift
public struct ImageReport: Sendable {
    public let orphans: [URL]              // image files not referenced
    public let brokenLinks: [(URL, String)] // (note URL, link string)
    public let absolutePathLinks: [(URL, String)]
}

public struct MigrationResult: Sendable {
    public let moved: Int
    public let rewritten: Int
}

public actor NotesImageTools {
    public init(root: URL)
    public func scan() async -> ImageReport
    public func migrateToAssets(folderName: String = "_assets") async throws -> MigrationResult
    public func checkTyporaConfig() async -> [String]   // human-readable warnings
}
```

Rules:

- Image extensions: `png`, `jpg`, `jpeg`, `gif`, `webp`, `svg`, `heic`.
- Parse `.md` files line-by-line; match `![alt](path)` and `<img src="path">` only. No HTML attribute parsing beyond `src="..."`.
- Resolve relative paths against the note's directory.
- Migrate: move each referenced image into `<root>/<folderName>/`, rewrite all references to the new relative path.
- Typora config check: read `~/Library/Preferences/abnerworks.Typora.plist`; warn if `Copy Image To Custom Folder` is absolute or empty.

### 8.2 NotesImageToolsPanel

File:

- `Sources/LumaApp/Launcher/NotesImageToolsPanel.swift`

Required UI:

- Modal sheet anchored to the launcher panel (`beginSheet`).
- Four buttons stacked: `Scan Orphans`, `Scan Broken Links`, `Migrate Images to _assets/`, `Check Typora Config`.
- Results list below: 12 pt monospaced text, scrollable.
- `Close` button at bottom right.

Tasks:

- Each button runs the corresponding tool in a `Task`, disables all buttons during execution, re-enables on completion.
- Migrate shows an extra confirmation alert listing the count to be moved before performing the operation.

Acceptance:

- Scan completes within 5 s on a 1000-file vault.
- Migration is idempotent: a second run reports 0 moved.
- Migration is reversible by `git` (no file deletions; only moves and edits).

## Phase 9: Polish (P2)

### 9.1 Wiki link jump

File:

- `Sources/LumaModules/Notes/NotesActions.swift`

Tasks:

- Add `relatedNotes(in note: URL) async -> [URL]`: parse the note body for `[[Title]]` mentions, match by exact filename (`Title.md`), return unique URLs.
- Expose a `Cmd+L` menu item in the detail view's context menu: `Open Linked Notes…`. Show a small popover listing matches; Return opens the selected one in Typora.

Acceptance:

- A note containing `[[Tree]] and [[DP]]` opens a popover with both entries.
- No popover if there are no matches.

### 9.2 Recent notes shortcut

File:

- `Sources/LumaModules/Notes/NotesModule.swift`

Tasks:

- Track the last 8 notes opened via `perform`. Persist in `notes.json` under `recent: [path]` (push to front, dedupe, cap at 8).
- When the launcher query is empty and the user is currently in the Notes detail view, surface the recent list at the top of the outline as a flat "Recent" pseudo-group (collapsible, default collapsed).

Acceptance:

- Opening eight notes populates the Recent group.
- Recent persists across app restarts.

## Phase 10: Docs and Verification

### 10.1 Performance spec

File:

- `docs/specs/PERFORMANCE.md`

Tasks:

- Add a row in the Latency Targets table:
  | `NotesModule.handle` | ≤ 10 ms | ≤ 30 ms | 40 ms (manifest queryTimeout) |
- Add a Hot Path Rules bullet: "Notes module reads happen only in `warmup` and on FSEvents callbacks. `handle` never touches disk."

### 10.2 Non-goals

File:

- `docs/NON_GOALS.md`

Tasks:

- Append a `Notes Module` subsection mirroring ADR-008's Explicit Non-Goals.

### 10.3 Manual QA checklist

File:

- `docs/MANUAL_QA_CHECKLIST.md`

Tasks:

- Append a `Notes v0.1` block:
  - Root picker writes `notes.json`.
  - Launcher hit: type `tree`, Return opens `Tree.md` in Typora.
  - Detail tree loads, default expansion is root only.
  - Create / rename / delete (note + empty folder) round-trip.
  - Non-empty folder delete is refused with the expected message.
  - External `mkdir` in the root surfaces in the tree within 1 second.
  - Image tools panel: scan + migrate + Typora config check.
  - Typora not installed: open falls back to `NSWorkspace.open` with no prompt.

### 10.4 ROADMAP

File:

- `docs/ROADMAP.md`

Task:

- Add a Notes v0.1 milestone entry with the eleven phases listed above.

## Full Acceptance Checklist

| # | Check |
| --- | --- |
| 1 | `NotesGraphIndexer.swift` and `NotesVaultStore.swift` are deleted; `Features/NotesGraph/README.md` is a one-line pointer to ADR-008. |
| 2 | `FSEventsService` watches a directory, debounces 200 ms, releases on stop. |
| 3 | `NotesTreeIndex.warmup` completes under 1 s on a 1000-file vault. |
| 4 | `NotesModule.handle` performs zero disk I/O and returns under 30 ms p95 warm. |
| 5 | Launcher search bar hits Notes filenames; Return opens in Typora. |
| 6 | Detail view renders tree with root expanded only; expansion state persists. |
| 7 | Create note auto-appends `.md`; same-name create is refused. |
| 8 | Rename and trash work for notes and empty folders; non-empty folder delete is refused. |
| 9 | External `.md` creation in root surfaces in the tree within 1 s. |
| 10 | In-tree filter highlights matches and expands ancestors. |
| 11 | Image tools panel runs all four commands; migration is idempotent. |
| 12 | Esc in detail returns to grid; Esc on grid closes panel (ADR-007 preserved). |
| 13 | `swift build -c release` is warning-free across new files. |
| 14 | Hot-path p95 from `LatencyHUD`-equivalent log surface is unchanged after Notes ships. |

## Non-Goals Reminder

If Cursor proposes any of the following, refuse and point it at ADR-008:

- Markdown rendering, preview pane, in-app editor.
- Page embedding / transclusion / `![[Note]]` rendering.
- Wiki-link backlinks (forward links via `[[…]]` jump are allowed in Phase 9; **backlinks are not**).
- Tags, frontmatter parsing, frontmatter generation, templates.
- Full-text body search.
- AI summarisation, AI Q&A, semantic search.
- Multi-vault, vault sync, vault import/export, version history.
- Drag-to-move tree nodes.
- Image gallery, image compression, image format conversion, paste interception.
- Inbox / orphan / recent counts surfaced on the dashboard card (recent group inside the detail view is fine; surfacing on the card is not).
