import Foundation
import Testing
import LumaCore

@Test func activitySnapshotProjectRecentIndependentOfGlobalTopEight() {
    let projectPath = "/Users/dev/Luma"
    let otherPath = "/Users/dev/Other"
    var entries: [WorkbenchActivityEntry] = []
    for index in 0..<9 {
        entries.append(
            WorkbenchActivityEntry(
                kind: .created,
                moduleID: .snippets,
                title: "Global \(index)",
                project: WorkbenchProjectAssociation(
                    projectPath: otherPath,
                    projectLabel: "Other"
                )
            )
        )
    }
    let projectEntry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Project-only activity",
        project: WorkbenchProjectAssociation(projectPath: projectPath, projectLabel: "Luma")
    )
    entries.append(projectEntry)

    let snapshot = WorkbenchActivitySnapshot.build(
        entries: entries,
        currentProjectPath: projectPath
    )

    #expect(snapshot.globalRecent.count == 8)
    #expect(!snapshot.globalRecent.contains(where: { $0.id == projectEntry.id }))
    #expect(snapshot.currentProjectRecent.count == 1)
    #expect(snapshot.currentProjectRecent[0].id == projectEntry.id)
}

@Test func projRecentUsesCurrentProjectSnapshotNotGlobalFilter() {
    let projectPath = "/Users/dev/Luma"
    let projectEntry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Hidden from global top 8",
        project: WorkbenchProjectAssociation(projectPath: projectPath, projectLabel: "Luma")
    )
    let globalOnly: [WorkbenchActivityEntry] = (0..<8).map { index in
        WorkbenchActivityEntry(
            kind: .created,
            moduleID: .todo,
            title: "Other \(index)",
            project: WorkbenchProjectAssociation(
                projectPath: "/tmp/other",
                projectLabel: "Other"
            )
        )
    }

    let context = WorkbenchContext(
        currentProject: CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.cursor",
            windowTitle: "Luma",
            projectLabel: "Luma",
            filename: nil,
            matchedProjectPath: projectPath
        ),
        enabledModuleIDs: [.projects, .snippets],
        pinnedModuleIDs: [.projects],
        activitySnapshot: WorkbenchActivitySnapshot(
            globalRecent: globalOnly,
            currentProjectRecent: [projectEntry]
        )
    )

    let rows = WorkbenchCommandResults.previewRows(
        route: .projectRecent,
        querySequence: 0,
        context: context
    )
    #expect(rows.count == 1)
    #expect(rows[0].title == "Hidden from global top 8")
}

@Test func projRecentFiltersDisabledModuleRows() {
    let projectPath = "/Users/dev/Luma"
    let snippetEntry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Disabled snippet draft",
        project: WorkbenchProjectAssociation(projectPath: projectPath, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let context = WorkbenchContext(
        currentProject: CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.cursor",
            windowTitle: "Luma",
            projectLabel: "Luma",
            filename: nil,
            matchedProjectPath: projectPath
        ),
        enabledModuleIDs: [.projects],
        pinnedModuleIDs: [.projects],
        activitySnapshot: WorkbenchActivitySnapshot(
            currentProjectRecent: [snippetEntry]
        )
    )

    let rows = WorkbenchCommandResults.previewRows(
        route: .projectRecent,
        querySequence: 0,
        context: context
    )
    #expect(rows.count == 1)
    #expect(rows[0].title == "Project recent activity")
    #expect(rows[0].subtitle == WorkbenchEmptyStateCopy.noRecentActivity)
}

@Test func projectIdentityPrefersMatchedPathForActivityQuery() {
    let identity = WorkbenchProjectIdentity(
        matchedPath: "/Users/dev/Luma",
        labelFallback: "Luma"
    )
    #expect(identity.activityQueryKey == "/Users/dev/Luma")

    let unmatched = WorkbenchProjectIdentity(matchedPath: nil, labelFallback: "Luma")
    #expect(unmatched.activityQueryKey == "Luma")
}

@Test func activitySnapshotCurrentProjectDraftsIndependentOfGlobalRecent() {
    let projectPath = "/Users/dev/Luma"
    let draft = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Stale draft",
        project: WorkbenchProjectAssociation(projectPath: projectPath, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let global = (0..<8).map { index in
        WorkbenchActivityEntry(
            kind: .created,
            moduleID: .todo,
            title: "Global \(index)"
        )
    }
    let snapshot = WorkbenchActivitySnapshot.build(
        entries: global + [draft],
        currentProjectPath: projectPath
    )
    #expect(snapshot.currentProjectDrafts.count == 1)
    #expect(snapshot.currentProjectDrafts[0].title == "Stale draft")
}
