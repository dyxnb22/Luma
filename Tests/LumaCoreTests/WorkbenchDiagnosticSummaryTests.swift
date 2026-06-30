import Foundation
import Testing
import LumaCore

@Test func workbenchDiagnosticSummaryBuildsFromContext() {
    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let link = WorkbenchProjectLink(
        stableProjectID: identity.stableProjectID,
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Snippet"
        )
    )
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Snippet",
        projectIdentity: identity
    )
    let context = WorkbenchContext(
        currentProject: CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.cursor",
            windowTitle: "Luma",
            projectLabel: "Luma",
            filename: nil,
            matchedProjectPath: "/tmp/luma"
        ),
        enabledModuleIDs: [.workbenchProjects, .workbenchSnippets],
        pinnedModuleIDs: [.workbenchProjects],
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry]),
        linkSnapshot: WorkbenchLinkSnapshot(currentProjectLinks: [link])
    )

    let summary = WorkbenchDiagnosticSummary.from(context: context)
    let expectedID = WorkbenchProjectIdentity(context: context.currentProject!).stableProjectID
    #expect(summary.stableProjectID == expectedID)
    #expect(summary.projectActivityCount == 1)
    #expect(summary.projectLinkCount == 1)
    #expect(summary.enabledProjectLinkCount == 1)
    #expect(summary.subtitle.contains("1 activities"))
    #expect(summary.subtitle.contains("1 indexed links"))
}

@Test func workbenchDiagnosticSummaryUsesFullProjectIndexCounts() {
    let identity = ProjectIdentity(
        stableProjectID: "path:/tmp/luma",
        matchedPath: "/tmp/luma",
        labelFallback: "Luma"
    )
    let entries = (0..<6).map { index in
        WorkbenchActivityEntry(
            kind: .projectLinked,
            moduleID: .workbenchSnippets,
            title: "Snippet \(index)",
            projectIdentity: identity
        )
    }
    let links = (0..<4).map { index in
        WorkbenchProjectLink(
            stableProjectID: identity.stableProjectID,
            entityRef: WorkbenchEntityRef(
                kind: .snippet,
                entityID: UUID().uuidString,
                moduleID: .workbenchSnippets,
                title: "Link \(index)"
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
            matchedProjectPath: "/tmp/luma"
        ),
        enabledModuleIDs: [.workbenchProjects, .workbenchSnippets],
        pinnedModuleIDs: [.workbenchProjects],
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: Array(entries.prefix(3))),
        linkSnapshot: WorkbenchLinkSnapshot(currentProjectLinks: Array(links.prefix(2))),
        projectIndexCounts: WorkbenchProjectIndexCounts(
            projectActivityCount: entries.count,
            projectLinkCount: links.count,
            enabledProjectLinkCount: links.count
        )
    )

    let summary = WorkbenchDiagnosticSummary.from(context: context)
    #expect(summary.projectActivityCount == 6)
    #expect(summary.projectLinkCount == 4)
    #expect(summary.fullMessage.contains("Project activities in store: 6"))
    #expect(summary.fullMessage.contains("Indexed links for project: 4"))
}

@Test func projStatusPreviewRowIsSideEffectFree() {
    let rows = WorkbenchCommandResults.previewRows(
        route: .projectStatus,
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
            enabledModuleIDs: [.workbenchProjects],
            pinnedModuleIDs: [.workbenchProjects]
        )
    )
    #expect(rows.count == 1)
    #expect(rows[0].title == "Project workbench status")
    guard case .custom(_, let handler) = rows[0].primaryAction.kind else {
        Issue.record("Expected command execute action")
        return
    }
    #expect(handler == .workbench)
}

@Test func projRecentEmptyUsesNoRecentActivityCopy() {
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
            enabledModuleIDs: [.workbenchProjects],
            pinnedModuleIDs: [.workbenchProjects]
        )
    )
    #expect(rows.count == 1)
    #expect(rows[0].subtitle == WorkbenchEmptyStateCopy.noRecentActivity)
}

@Test func projLinksEmptyUsesNoLinkedItemsCopy() {
    let rows = WorkbenchCommandResults.previewRows(
        route: .projectLinks,
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
            enabledModuleIDs: [.workbenchProjects],
            pinnedModuleIDs: [.workbenchProjects]
        )
    )
    #expect(rows.count == 1)
    #expect(rows[0].subtitle == WorkbenchEmptyStateCopy.noLinkedItems)
}
