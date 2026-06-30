import Foundation
import Testing
import LumaCore

@Test func legacyV1ActivityFileMigratesToV2() async throws {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-migrate-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let legacyJSON = """
    {"version":1,"entries":[{"id":"\(UUID().uuidString)","kind":"projectLinked","moduleID":"luma.snippets","title":"Legacy snippet","project":{"projectPath":"/Users/dev/Luma","projectLabel":"Luma"},"resumePayloadJSON":null}]}
    """
    try legacyJSON.write(to: url, atomically: true, encoding: .utf8)

    let store = WorkbenchActivityStore(fileURL: url)
    let entries = await store.allEntries()
    #expect(entries.count == 1)
    #expect(entries[0].projectIdentity?.matchedPath == "/Users/dev/Luma")
    #expect(entries[0].project == nil)

    let data = try Data(contentsOf: url)
    let raw = String(data: data, encoding: .utf8) ?? ""
    #expect(raw.contains("\"version\":2"))
}

@Test func sameLabelDifferentPathsDoNotMix() {
    let label = "MyApp"
    let pathA = "/Users/dev/project-a"
    let pathB = "/Users/dev/project-b"
    let identityA = ProjectIdentity(context: CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "A",
        projectLabel: label,
        filename: nil,
        matchedProjectPath: pathA
    ))
    let identityB = ProjectIdentity(context: CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "B",
        projectLabel: label,
        filename: nil,
        matchedProjectPath: pathB
    ))
    #expect(identityA.stableProjectID != identityB.stableProjectID)

    let entries = [
        WorkbenchActivityEntry(
            kind: .projectLinked,
            moduleID: .workbenchSnippets,
            title: "A only",
            projectIdentity: identityA
        ),
        WorkbenchActivityEntry(
            kind: .projectLinked,
            moduleID: .workbenchSnippets,
            title: "B only",
            projectIdentity: identityB
        )
    ]
    let recentA = WorkbenchActivityQuery.recent(for: identityA, entries: entries)
    #expect(recentA.count == 1)
    #expect(recentA[0].title == "A only")
}

@Test func unmatchedLegacyLabelStillQueryable() {
    let legacy = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Unmatched label activity",
        project: WorkbenchProjectAssociation(projectPath: "UnmatchedWindow", projectLabel: "UnmatchedWindow")
    )
    let migrated = WorkbenchActivityStore.migrateEntry(legacy)
    guard let migratedIdentity = migrated.projectIdentity else {
        Issue.record("Expected migrated project identity")
        return
    }
    #expect(ProjectIdentity.isLegacyLabelStableID(migratedIdentity.stableProjectID))

    let currentContext = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "UnmatchedWindow",
        projectLabel: "UnmatchedWindow",
        filename: nil,
        matchedProjectPath: nil
    )
    let queryIdentity = ProjectIdentity(context: currentContext)
    #expect(queryIdentity.stableProjectID != migratedIdentity.stableProjectID)

    let recent = WorkbenchActivityQuery.recent(for: queryIdentity, entries: [migrated])
    #expect(recent.count == 1)
}

@Test func v1FileStillResumesAfterUpgrade() {
    let entryID = UUID()
    let payload = WorkbenchActivityResumePayload.snippetDraft(Data("{\"trigger\":\"x\"}".utf8)).encoded()
    let legacy = WorkbenchActivityEntry(
        id: entryID,
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Resume me",
        project: WorkbenchProjectAssociation(projectPath: "/tmp/luma", projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: entryID),
        resumePayloadJSON: payload
    )
    let migrated = WorkbenchActivityStore.migrateEntry(legacy)
    #expect(migrated.resumablePayload != nil)
    #expect(migrated.isResumableDraft)
    #expect(migrated.id == entryID)
}

@Test func captureWritesActivityAndLink() async {
    let activityURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-cap-\(UUID().uuidString).json")
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-\(UUID().uuidString).json")
    defer {
        try? FileManager.default.removeItem(at: activityURL)
        try? FileManager.default.removeItem(at: linkURL)
    }

    let activityStore = WorkbenchActivityStore(fileURL: activityURL)
    let linkStore = WorkbenchLinkStore(fileURL: linkURL)
    let project = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: nil,
        matchedProjectPath: "/Users/dev/Luma"
    )
    let context = WorkbenchContext(
        currentProject: project,
        enabledModuleIDs: [.workbenchSnippets, .workbenchProjects],
        pinnedModuleIDs: [.workbenchProjects]
    )
    let result = WorkbenchCaptureResult(
        target: .projectSnippetDraft,
        moduleID: .snippets,
        preview: "test-snippet",
        openDetailPayload: Data(),
        resumeDraftJSON: Data("{}".utf8)
    )
    let entry = await activityStore.recordCapture(
        result: result,
        context: context,
        attribution: WorkbenchCaptureAttribution(sourceKind: .home)
    )
    let identity = ProjectIdentity(context: project)
    await linkStore.recordLink(for: entry, identity: identity)

    let links = await linkStore.links(for: identity.stableProjectID)
    #expect(links.count == 1)
    #expect(links[0].activityEntryID == entry.id)
}

@Test func disabledModuleFiltersLinkedRows() {
    let link = WorkbenchProjectLink(
        stableProjectID: "path:test",
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Snippet"
        )
    )
    let snapshot = WorkbenchLinkSnapshot(currentProjectLinks: [link])
    let gated = snapshot.enabledLinks(enabledModuleIDs: [.workbenchProjects])
    #expect(gated.isEmpty)
    let allowed = snapshot.enabledLinks(enabledModuleIDs: [.workbenchProjects, .workbenchSnippets])
    #expect(allowed.count == 1)
}
