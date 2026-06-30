import Foundation
import Testing
import LumaCore

@Test func workspaceModelBuilderOrdersQuickCaptureBeforeProjectActions() {
    let path = "/Users/dev/Luma"
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: "main.swift",
        matchedProjectPath: path
    )
    let snapshot = WorkbenchActivitySnapshot(
        currentProjectRecent: [
            WorkbenchActivityEntry(
                kind: .created,
                moduleID: .snippets,
                title: "Saved snippet",
                project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma")
            )
        ]
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: snapshot,
        enabledModuleIDs: [.snippets, .quicklinks, .todo, .notes, .projects]
    )

    #expect(model.showsProjectActions)
    #expect(!model.quickCaptureActions.isEmpty)
    #expect(model.quickCaptureDisabledHint == nil)
    #expect(model.recentActivityLines.count == 1)
    #expect(model.recentActivityLines[0].title == "Saved snippet")
}

@Test func workspaceModelBuilderHidesDisabledCaptureModules() {
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: nil,
        matchedProjectPath: "/Users/dev/Luma"
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: WorkbenchActivitySnapshot(),
        enabledModuleIDs: [.projects]
    )

    #expect(model.quickCaptureActions.isEmpty)
    #expect(model.quickCaptureDisabledHint?.contains("Enable Snippets") == true)
}

@Test func workspaceModelBuilderOmitsQuickCaptureWithoutMatchedPath() {
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Unmatched",
        projectLabel: "Unmatched",
        filename: nil,
        matchedProjectPath: nil
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: WorkbenchActivitySnapshot(),
        enabledModuleIDs: [.snippets, .projects]
    )

    #expect(model.quickCaptureActions.isEmpty)
    #expect(model.quickCaptureDisabledHint == nil)
    #expect(!model.showsProjectActions)
}

@Test func workspaceModelBuilderUsesSnapshotNotGlobalRecent() {
    let path = "/Users/dev/Luma"
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: nil,
        matchedProjectPath: path
    )
    let projectOnly = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Not in global eight",
        project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma")
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: WorkbenchActivitySnapshot(
            globalRecent: [],
            currentProjectRecent: [projectOnly]
        ),
        enabledModuleIDs: [.projects, .snippets]
    )
    #expect(model.recentActivityLines.count == 1)
    #expect(model.recentActivityLines[0].title == "Not in global eight")
}

@Test func workspaceModelBuilderLoadingClearsPriorSections() {
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: nil,
        matchedProjectPath: "/Users/dev/Luma"
    )
    let model = CurrentProjectWorkspaceModelBuilder.loading(context: context)
    #expect(model.headerLines == ["Loading Luma…"])
    #expect(model.quickCaptureActions.isEmpty)
    #expect(model.recentActivityLines.isEmpty)
    #expect(!model.showsProjectActions)
}
