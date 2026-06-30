import Foundation
import Testing
import LumaCore

@Test func linkStoreBackfillsFromActivitiesWhenEmpty() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-backfill-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Legacy snippet",
        projectIdentity: identity,
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )

    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.backfillFromActivitiesIfEmpty([entry])

    let links = await store.links(for: identity.stableProjectID)
    #expect(links.count == 1)
    #expect(links[0].activityEntryID == entry.id)
    #expect(links[0].entityRef.kind == .snippet)
}

@Test func linkStoreBackfillSkipsWhenLinksAlreadyExist() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-skip-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let store = WorkbenchLinkStore(fileURL: linkURL)
    let ref = WorkbenchEntityRef(
        kind: .snippet,
        entityID: UUID().uuidString,
        moduleID: .workbenchSnippets,
        title: "Existing"
    )
    await store.recordLink(
        stableProjectID: "path:/tmp/luma",
        entityRef: ref,
        activityEntryID: nil,
        sourceKind: nil
    )

    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchTodo,
        title: "Todo",
        projectIdentity: ProjectIdentity(
            stableProjectID: "path:/tmp/luma",
            matchedPath: "/tmp/luma",
            labelFallback: "Luma"
        ),
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Todo").encoded()
    )
    await store.backfillFromActivitiesIfEmpty([entry])

    let links = await store.allLinks()
    #expect(links.count == 1)
    #expect(links[0].entityRef.kind == .snippet)
}

@Test func linkStoreMatchesLegacyLabelFallback() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-legacy-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let legacyID = ProjectIdentity.makeStableID(
        matchedPath: nil,
        labelFallback: "UnmatchedWindow",
        sourceBundleID: nil
    )
    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.recordLink(
        stableProjectID: legacyID,
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Legacy"
        ),
        activityEntryID: UUID(),
        sourceKind: nil,
        labelFallback: "UnmatchedWindow"
    )

    let queryIdentity = ProjectIdentity(context: CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "UnmatchedWindow",
        projectLabel: "UnmatchedWindow",
        filename: nil,
        matchedProjectPath: nil
    ))
    let links = await store.snapshot(for: queryIdentity)
    #expect(links.count == 1)
}
