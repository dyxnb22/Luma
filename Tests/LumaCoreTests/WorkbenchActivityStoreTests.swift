import Foundation
import Testing
import LumaCore

@Test func legacyActivityJSONDecodesWithoutNewFields() async throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-legacy-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let legacyJSON = """
    {"entries":[{"id":"\(UUID().uuidString)","kind":"draftPrepared","moduleID":"luma.snippets","entityKind":"snippet","title":"Prepared snippet draft","detail":"hello","recordedAt":0}]}
    """
    try legacyJSON.write(to: url, atomically: true, encoding: .utf8)

    let store = WorkbenchActivityStore(fileURL: url)
    let snapshot = await store.snapshot(limit: 1)
    #expect(snapshot.count == 1)
    #expect(snapshot[0].project == nil)
    #expect(snapshot[0].resumeRef == nil)
    #expect(snapshot[0].sourceKind == nil)
}

@Test func activityEnvelopeRoundTripsV1Schema() async throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-v1-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = WorkbenchActivityStore(fileURL: url, maxEntries: 50)
    let project = WorkbenchProjectAssociation(
        projectPath: "/Users/dev/Luma",
        projectLabel: "Luma",
        projectName: "Luma"
    )
    await store.record(WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        entityKind: .snippet,
        title: "Project snippet",
        detail: "slug",
        project: project,
        sourceKind: .home,
        actionKind: WorkbenchCaptureFollowUp.openDetail.rawValue,
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        preview: "slug"
    ))

    let data = try Data(contentsOf: url)
    let raw = String(data: data, encoding: .utf8) ?? ""
    #expect(raw.contains("\"version\":2"))

    let reloaded = WorkbenchActivityStore(fileURL: url)
    let entries = await reloaded.allEntries()
    #expect(entries.count == 1)
    #expect(entries[0].projectIdentity?.matchedPath == "/Users/dev/Luma")
    #expect(entries[0].sourceKind == .home)
}

@Test func activityMaxEntriesCapStillWorks() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-cap-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = WorkbenchActivityStore(fileURL: url, maxEntries: 3)
    for index in 0..<5 {
        await store.record(WorkbenchActivityEntry(
            kind: .draftPrepared,
            moduleID: .workbenchNotes,
            title: "Entry \(index)"
        ))
    }
    let all = await store.allEntries()
    #expect(all.count == 3)
    #expect(all[0].title == "Entry 4")
}

@Test func activityQueryFiltersByProjectAndModule() async {
    let pathA = "/tmp/project-a"
    let pathB = "/tmp/project-b"
    let entries = [
        WorkbenchActivityEntry(
            kind: .draftPrepared,
            moduleID: .workbenchSnippets,
            title: "A snippet",
            project: WorkbenchProjectAssociation(projectPath: pathA, projectLabel: "A"),
            resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
            resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
        ),
        WorkbenchActivityEntry(
            kind: .draftPrepared,
            moduleID: .workbenchTodo,
            title: "B todo",
            project: WorkbenchProjectAssociation(projectPath: pathB, projectLabel: "B"),
            resumeRef: WorkbenchResumeRef(kind: .todoCapture, entryID: UUID()),
            resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("todo").encoded()
        ),
        WorkbenchActivityEntry(
            kind: .draftPrepared,
            moduleID: .workbenchSnippets,
            title: "No project"
        )
    ]

    let projectA = WorkbenchActivityQuery.recent(forProject: pathA, entries: entries)
    #expect(projectA.count == 1)
    #expect(projectA[0].title == "A snippet")

    let drafts = WorkbenchActivityQuery.recentDrafts(forProject: pathA, entries: entries)
    #expect(drafts.count == 1)

    let snippets = WorkbenchActivityQuery.recentByModule(.workbenchSnippets, entries: entries)
    #expect(snippets.count == 2)

    let latest = WorkbenchActivityQuery.latestProjectContext(entries: entries)
    #expect(latest?.projectPath == pathA)
}

@Test func corruptActivityFileLoadsEmptyStore() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-bad-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    try? Data("{not json".utf8).write(to: url)
    let store = WorkbenchActivityStore(fileURL: url)
    let snapshot = await store.snapshot()
    #expect(snapshot.isEmpty)
}

@Test func captureRecordsProjectAttributionWhenProjectExists() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-attr-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = WorkbenchActivityStore(fileURL: url)
    let project = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma — main.swift",
        projectLabel: "Luma",
        filename: "main.swift",
        matchedProjectPath: "/Users/dev/Luma",
        matchedProjectName: "Luma"
    )
    let context = WorkbenchContext(
        currentProject: project,
        enabledModuleIDs: [.workbenchSnippets],
        pinnedModuleIDs: [.workbenchSnippets]
    )
    let result = WorkbenchCaptureResult(
        target: .projectSnippetDraft,
        moduleID: .snippets,
        preview: "luma-test",
        openDetailPayload: Data(),
        resumeDraftJSON: Data("{}".utf8)
    )
    await store.recordCapture(
        result: result,
        context: context,
        attribution: WorkbenchCaptureAttribution(sourceKind: .home, followUp: .openDetail)
    )
    let entry = await store.snapshot(limit: 1)[0]
    #expect(entry.projectIdentity?.matchedPath == "/Users/dev/Luma")
    #expect(entry.sourceKind == .home)
    #expect(entry.kind == .projectLinked)
    #expect(entry.resumeRef?.kind == .snippetDraft)
    #expect(entry.actionKind == WorkbenchCaptureFollowUp.openDetail.rawValue)
}

@Test func captureWithoutProjectOmitsProjectAssociation() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-no-proj-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = WorkbenchActivityStore(fileURL: url)
    let context = WorkbenchContext(
        enabledModuleIDs: [.workbenchTodo],
        pinnedModuleIDs: []
    )
    let result = WorkbenchCaptureResult(
        target: .todoDraft,
        moduleID: .todo,
        preview: "buy milk"
    )
    await store.recordCapture(
        result: result,
        context: context,
        attribution: WorkbenchCaptureAttribution(sourceKind: .command, followUp: .replaceQuery)
    )
    let entry = await store.snapshot(limit: 1)[0]
    #expect(entry.project == nil)
    #expect(entry.resumeRef?.kind == .todoCapture)
    #expect(entry.resumablePayload != nil)
}

@Test func captureStoresPerEntryResumePayload() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-payload-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = WorkbenchActivityStore(fileURL: url)
    let context = WorkbenchContext(
        enabledModuleIDs: [.workbenchSnippets],
        pinnedModuleIDs: []
    )
    let draftA = Data("{\"trigger\":\"proj-a\"}".utf8)
    let draftB = Data("{\"trigger\":\"proj-b\"}".utf8)
    await store.recordCapture(
        result: WorkbenchCaptureResult(
            target: .snippetDraft,
            moduleID: .snippets,
            preview: "proj-a",
            resumeDraftJSON: draftA
        ),
        context: context,
        attribution: WorkbenchCaptureAttribution(sourceKind: .home)
    )
    await store.recordCapture(
        result: WorkbenchCaptureResult(
            target: .snippetDraft,
            moduleID: .snippets,
            preview: "proj-b",
            resumeDraftJSON: draftB
        ),
        context: context,
        attribution: WorkbenchCaptureAttribution(sourceKind: .home)
    )
    let entries = await store.allEntries()
    #expect(entries.count == 2)
    #expect(entries[0].resumablePayload == .snippetDraft(draftB))
    #expect(entries[1].resumablePayload == .snippetDraft(draftA))
}

@Test func noteDraftExcludedFromRecentDrafts() async {
    let path = "/tmp/luma"
    let noteEntry = WorkbenchActivityEntry(
        kind: .draftPrepared,
        moduleID: .notes,
        title: "Prepared note draft",
        project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .noteAction, entryID: UUID())
    )
    let snippetEntry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Project snippet",
        project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let drafts = WorkbenchActivityQuery.recentDrafts(
        forProject: path,
        entries: [noteEntry, snippetEntry]
    )
    #expect(drafts.count == 1)
    #expect(drafts[0].moduleID == .snippets)
}

@Test func projRecentResumableRowEncodesResumeActivity() {
    let entryID = UUID()
    let entry = WorkbenchActivityEntry(
        id: entryID,
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Project snippet",
        project: WorkbenchProjectAssociation(projectPath: "/tmp/luma", projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: entryID),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let rows = WorkbenchCommandResults.previewRows(
        route: .projectRecent,
        querySequence: 0,
        context: WorkbenchContext(
            currentProject: CurrentProjectContext(
                frontAppName: "Cursor",
                bundleID: "com.cursor",
                windowTitle: "Luma",
                projectLabel: "Luma",
                filename: nil,
                matchedProjectPath: "/tmp/luma"
            ),
            enabledModuleIDs: [.projects, .snippets],
            pinnedModuleIDs: [.projects],
            activitySnapshot: WorkbenchActivitySnapshot(
                globalRecent: [entry],
                currentProjectRecent: [entry]
            )
        )
    )
    #expect(rows.count == 1)
    guard case .custom(let payload, let handler) = rows[0].primaryAction.kind,
          handler == .workbench else {
        Issue.record("Expected workbench custom action")
        return
    }
    let action = try? ModuleActionCoding.decode(WorkbenchCaptureAction.self, from: payload)
    guard case .resumeActivity(let id) = action else {
        Issue.record("Expected resumeActivity action")
        return
    }
    #expect(id == entryID)
}
