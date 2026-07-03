import Foundation
import Testing
import LumaCore

@Test func activityRowPresentationMatchesAcrossSurfaces() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchTodo,
        title: "Fix tests",
        preview: "Fix tests",
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Fix tests").encoded()
    )
    let presentation = WorkbenchActivityRowActions.presentation(for: entry)
    let detailRow = CurrentProjectWorkspaceModelBuilder.build(
        context: CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.cursor",
            windowTitle: "Luma",
            projectLabel: "Luma",
            filename: nil,
            matchedProjectPath: "/tmp/luma"
        ),
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry]),
        enabledModuleIDs: [.workbenchProjects, .workbenchTodo]
    ).recentActivityRows[0]

    #expect(presentation.rowAction == detailRow.action)
    #expect(presentation.subtitle == detailRow.subtitle)
    #expect(presentation.isInteractive == detailRow.isInteractive)
}

@Test func homeRecentTodoRowIsNotNoop() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchTodo,
        title: "Fix tests",
        preview: "Fix tests",
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Fix tests").encoded()
    )
    let action = WorkbenchActivityRowActions.primaryAction(
        for: entry,
        key: "workbench.recent"
    )
    guard case .replaceQuery = action.kind else {
        Issue.record("Expected replaceQuery for todo capture row")
        return
    }
}

@Test func homeRecentSnippetRowIsNotNoop() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Saved snippet",
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Saved snippet"
        )
    )
    let action = WorkbenchActivityRowActions.primaryAction(
        for: entry,
        key: "workbench.recent"
    )
    guard case .openModuleDetail = action.kind else {
        Issue.record("Expected openModuleDetail for linked snippet row")
        return
    }
}

@Test func recordedActivityRowEncodesShowStatus() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchProjects,
        title: "Opened workspace",
        preview: "Viewed project"
    )
    let presentation = WorkbenchActivityRowActions.presentation(for: entry)
    #expect(!presentation.isInteractive)
    #expect(presentation.subtitle == "Viewed project")

    let action = WorkbenchActivityRowActions.primaryAction(for: entry, key: "workbench.recent")
    guard case .custom(let payload, let handler) = action.kind,
          handler == .workbench else {
        Issue.record("Expected workbench showStatus action")
        return
    }
    let entityAction = try? ModuleActionCoding.decode(WorkbenchEntityAction.self, from: payload)
    guard case .showStatus(let message) = entityAction else {
        Issue.record("Expected showStatus payload")
        return
    }
    #expect(message == "Recorded activity")
}

@Test func nonOpenableActivityRowPresentationConsistentAcrossSurfaces() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchProjects,
        title: "Opened workspace",
        preview: "Viewed project"
    )
    let presentation = WorkbenchActivityRowActions.presentation(for: entry)
    let detailRow = CurrentProjectWorkspaceModelBuilder.build(
        context: CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.cursor",
            windowTitle: "Luma",
            projectLabel: "Luma",
            filename: nil,
            matchedProjectPath: "/tmp/luma"
        ),
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry]),
        enabledModuleIDs: [.workbenchProjects]
    ).recentActivityRows[0]

    #expect(presentation.subtitle == detailRow.subtitle)
    #expect(presentation.isInteractive == detailRow.isInteractive)

    let commandRows = WorkbenchCommandResults.previewRows(
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
            pinnedModuleIDs: [.workbenchProjects],
            activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry])
        )
    )
    #expect(commandRows.count == 1)
    #expect(commandRows[0].subtitle == presentation.subtitle)
}

@Test func workspaceRowActionCodecTodoCaptureCommandOutcome() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchTodo,
        title: "Fix tests",
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Fix tests").encoded()
    )
    let rowAction = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry)
    guard case .replaceQuery(let query) = rowAction else {
        Issue.record("Expected replaceQuery row action")
        return
    }
    let outcome = WorkbenchWorkspaceRowActionCodec.commandOutcome(for: rowAction, entry: entry)
    guard case .replaceQuery(let outcomeQuery) = outcome else {
        Issue.record("Expected replaceQuery command outcome")
        return
    }
    #expect(outcomeQuery == query)
}

@Test func disabledModuleActivityRowStillBuildsNoopAction() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Snippet",
        entityRef: WorkbenchEntityRef(
            kind: .snippet,
            entityID: UUID().uuidString,
            moduleID: .workbenchSnippets,
            title: "Snippet"
        )
    )
    let snapshot = WorkbenchActivitySnapshot(currentProjectRecent: [entry])
    let gated = snapshot.enabledCurrentProjectRecent(
        enabledModuleIDs: [.workbenchProjects],
        limit: 3
    )
    #expect(gated.isEmpty)
}

@Test func linkedEntityPlannerNoteReferenceOpensNotesModule() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchNotes,
        title: "Daily note",
        resumePayloadJSON: WorkbenchActivityResumePayload.noteReference(
            path: "/tmp/daily.md",
            title: "Daily"
        ).encoded()
    )
    let action = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry)
    guard case .openModule(let moduleID) = action else {
        Issue.record("Expected openModule for noteReference payload")
        return
    }
    #expect(moduleID == .workbenchNotes)
}
