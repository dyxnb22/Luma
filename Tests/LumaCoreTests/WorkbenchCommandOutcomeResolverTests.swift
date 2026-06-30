import Foundation
import Testing
import LumaCore

@Test func projectRecentBareCommandReturnsEmptyStatusWhenNoActivity() {
    let context = WorkbenchContext(
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
    let outcome = WorkbenchCommandOutcomeResolver.projectRecent(context: context)
    guard case .status(let message) = outcome else {
        Issue.record("Expected status outcome for empty recent")
        return
    }
    #expect(message == WorkbenchEmptyStateCopy.noRecentActivity)
}

@Test func projectLinksBareCommandReturnsEmptyStatusWhenNoLinks() {
    let context = WorkbenchContext(
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
    let outcome = WorkbenchCommandOutcomeResolver.projectLinks(context: context)
    guard case .status(let message) = outcome else {
        Issue.record("Expected status outcome for empty links")
        return
    }
    #expect(message == WorkbenchEmptyStateCopy.noLinkedItems)
}

@Test func projectRecentBareCommandReturnsStatusForNonOpenableActivity() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchProjects,
        title: "Opened workspace",
        projectIdentity: ProjectIdentity(
            stableProjectID: "path:/tmp/luma",
            matchedPath: "/tmp/luma",
            labelFallback: "Luma"
        ),
        preview: "Viewed project"
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
        enabledModuleIDs: [.workbenchProjects],
        pinnedModuleIDs: [.workbenchProjects],
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry])
    )
    let outcome = WorkbenchCommandOutcomeResolver.projectRecent(context: context)
    guard case .status(let message) = outcome else {
        Issue.record("Expected status outcome for non-openable activity")
        return
    }
    #expect(message == "Recorded activity")
}

@Test func projectRecentBareCommandReturnsReplaceQueryForTodoCapture() {
    let entry = WorkbenchActivityEntry(
        kind: .draftPrepared,
        moduleID: .workbenchTodo,
        title: "Fix tests",
        projectIdentity: ProjectIdentity(
            stableProjectID: "path:/tmp/luma",
            matchedPath: "/tmp/luma",
            labelFallback: "Luma"
        ),
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Fix tests").encoded()
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
        enabledModuleIDs: [.workbenchProjects, .workbenchTodo],
        pinnedModuleIDs: [.workbenchProjects],
        activitySnapshot: WorkbenchActivitySnapshot(currentProjectRecent: [entry])
    )
    let outcome = WorkbenchCommandOutcomeResolver.projectRecent(context: context)
    guard case .replaceQuery(let query) = outcome else {
        Issue.record("Expected replaceQuery outcome for todo capture")
        return
    }
    #expect(query == TodoModuleResumeQuery.resumeQuery(forCapture: "Fix tests"))
}
