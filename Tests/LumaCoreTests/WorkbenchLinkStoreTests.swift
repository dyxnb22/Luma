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

@Test func linkStoreBackfillsWhenProjectLinksEmptyButStoreNotEmpty() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-project-empty-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let otherIdentity = ProjectIdentity(
        stableProjectID: "path:/tmp/other",
        matchedPath: "/tmp/other",
        labelFallback: "Other"
    )
    let targetIdentity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )

    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.recordLink(
        stableProjectID: otherIdentity.stableProjectID,
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Other project"
        ),
        activityEntryID: nil,
        sourceKind: nil
    )

    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Legacy snippet",
        projectIdentity: targetIdentity,
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )

    await store.ensureLinksIndexed(for: targetIdentity, from: [entry])

    let targetLinks = await store.snapshot(for: targetIdentity)
    #expect(targetLinks.count == 1)
    #expect(targetLinks[0].activityEntryID == entry.id)
}

@Test func linkStoreBackfillRespectsMaxLinksCap() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-cap-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let entries = (0..<110).map { index in
        WorkbenchActivityEntry(
            kind: .projectLinked,
            moduleID: .workbenchSnippets,
            title: "Snippet \(index)",
            projectIdentity: identity,
            entityRef: WorkbenchEntityRef(
                kind: .snippet,
                entityID: UUID().uuidString,
                moduleID: .workbenchSnippets,
                title: "Snippet \(index)"
            )
        )
    }

    let store = WorkbenchLinkStore(fileURL: linkURL, maxLinks: 100)
    await store.ensureLinksIndexed(for: nil, from: entries)

    let all = await store.allLinks()
    #expect(all.count == 100)
}

@Test func linkStoreBackfillDedupesStableProjectIDAndEntityRef() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-dedupe-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let entityID = UUID().uuidString
    let ref = WorkbenchEntityRef(
        kind: .snippet,
        entityID: entityID,
        moduleID: .workbenchSnippets,
        title: "Same snippet"
    )
    let entries = [
        WorkbenchActivityEntry(
            kind: .projectLinked,
            moduleID: .workbenchSnippets,
            title: "Newer",
            recordedAt: Date(timeIntervalSince1970: 200),
            projectIdentity: identity,
            entityRef: ref
        ),
        WorkbenchActivityEntry(
            kind: .projectLinked,
            moduleID: .workbenchSnippets,
            title: "Older",
            recordedAt: Date(timeIntervalSince1970: 100),
            projectIdentity: identity,
            entityRef: ref
        )
    ]

    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.ensureLinksIndexed(for: nil, from: entries)

    let links = await store.links(for: identity.stableProjectID)
    #expect(links.count == 1)
    #expect(links[0].entityRef.entityID == entityID)
}

@Test func linkStoreBackfillsDraftPreparedProjectActivity() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-draft-project-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let entry = WorkbenchActivityEntry(
        kind: .draftPrepared,
        moduleID: .workbenchTodo,
        title: "Project todo",
        projectIdentity: identity,
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Ship feature").encoded()
    )

    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.ensureLinksIndexed(for: nil, from: [entry])

    let links = await store.allLinks()
    #expect(links.count == 1)
    #expect(links[0].entityRef.kind == .todo)
}

@Test func linkStoreBackfillSkipsDraftPreparedWithoutProjectIdentity() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-draft-global-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let entry = WorkbenchActivityEntry(
        kind: .draftPrepared,
        moduleID: .workbenchSnippets,
        title: "Draft only",
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )

    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.ensureLinksIndexed(for: nil, from: [entry])

    let links = await store.allLinks()
    #expect(links.isEmpty)
}

@Test func recordLinkUpdatesTitleWithoutDuplicate() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-title-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let entityID = UUID().uuidString
    let store = WorkbenchLinkStore(fileURL: linkURL)
    let initialRef = WorkbenchEntityRef(
        kind: .snippet,
        entityID: entityID,
        moduleID: .workbenchSnippets,
        title: "Snippet",
        subtitle: "Old preview"
    )
    await store.recordLink(
        stableProjectID: "path:/tmp/luma",
        entityRef: initialRef,
        activityEntryID: UUID(),
        sourceKind: nil
    )

    let updatedRef = WorkbenchEntityRef(
        kind: .snippet,
        entityID: entityID,
        moduleID: .workbenchSnippets,
        title: "Snippet",
        subtitle: "New preview"
    )
    await store.recordLink(
        stableProjectID: "path:/tmp/luma",
        entityRef: updatedRef,
        activityEntryID: UUID(),
        sourceKind: nil
    )

    let links = await store.allLinks()
    #expect(links.count == 1)
    #expect(links[0].entityRef.subtitle == "New preview")
}

@Test func recordLinkAndBackfillShareDedupeKey() async {
    let linkURL = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-links-shared-dedupe-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: linkURL) }

    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let entityID = UUID().uuidString
    let entry = WorkbenchActivityEntry(
        id: UUID(),
        kind: .draftPrepared,
        moduleID: .workbenchSnippets,
        title: "Snippet",
        entityID: entityID,
        projectIdentity: identity,
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: entityID,
            moduleID: .workbenchSnippets,
            title: "Snippet",
            subtitle: "From activity"
        )
    )

    let store = WorkbenchLinkStore(fileURL: linkURL)
    await store.ensureLinksIndexed(for: nil, from: [entry])
    await store.recordLink(for: entry, identity: identity)

    let links = await store.allLinks()
    #expect(links.count == 1)
    #expect(links[0].activityEntryID == entry.id)
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
