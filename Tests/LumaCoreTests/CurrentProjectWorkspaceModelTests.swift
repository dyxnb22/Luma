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
        entityKind: .snippet,
        title: "Not in global eight",
        project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: WorkbenchActivitySnapshot(
            globalRecent: [],
            currentProjectRecent: [projectOnly]
        ),
        enabledModuleIDs: [.projects, .snippets]
    )
    #expect(model.recentActivityRows.count == 1)
    #expect(model.recentActivityRows[0].title == "Not in global eight")
    #expect(model.recentActivityRows[0].isInteractive)
}

@Test func workspaceModelBuilderTodoCaptureRowUsesReplaceQuery() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchTodo,
        title: "Fix tests",
        preview: "Fix tests",
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Fix tests").encoded()
    )
    let path = "/Users/dev/Luma"
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: nil,
        matchedProjectPath: path
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry]),
        enabledModuleIDs: [.workbenchProjects, .workbenchTodo]
    )
    #expect(model.recentActivityRows.count == 1)
    guard case .replaceQuery(let query) = model.recentActivityRows[0].action else {
        Issue.record("Expected replaceQuery action for todo capture row")
        return
    }
    #expect(query == TodoModuleResumeQuery.resumeQuery(forCapture: "Fix tests"))
}

@Test func workspaceModelBuilderNoteReferenceRowUsesOpenNotePath() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchNotes,
        title: "Daily note",
        resumePayloadJSON: WorkbenchActivityResumePayload.noteReference(
            path: "/Users/dev/notes/daily.md",
            title: "Daily"
        ).encoded()
    )
    let path = "/Users/dev/Luma"
    let context = CurrentProjectContext(
        frontAppName: "Cursor",
        bundleID: "com.cursor",
        windowTitle: "Luma",
        projectLabel: "Luma",
        filename: nil,
        matchedProjectPath: path
    )
    let model = CurrentProjectWorkspaceModelBuilder.build(
        context: context,
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry]),
        enabledModuleIDs: [.workbenchProjects, .workbenchNotes]
    )
    #expect(model.recentActivityRows.count == 1)
    guard case .openNotePath(let notePath) = model.recentActivityRows[0].action else {
        Issue.record("Expected openNotePath action for note reference row")
        return
    }
    #expect(notePath == "/Users/dev/notes/daily.md")
}

@Test func workspaceLinkedRowUsesOpenLinkedAction() {
    let linkID = UUID()
    let link = WorkbenchProjectLink(
        id: linkID,
        stableProjectID: "path:test",
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Linked snippet"
        ),
        activityEntryID: UUID()
    )
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
        linkSnapshot: WorkbenchLinkSnapshot(currentProjectLinks: [link]),
        enabledModuleIDs: [.workbenchProjects, .workbenchSnippets]
    )
    #expect(model.linkedItemRows.count == 1)
    guard case .openLinked(let id) = model.linkedItemRows[0].action else {
        Issue.record("Expected openLinked action for linked row")
        return
    }
    #expect(id == linkID)
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
    #expect(model.linkedItemRows.isEmpty)
    #expect(model.recentActivityRows.isEmpty)
    #expect(!model.showsProjectActions)
}
