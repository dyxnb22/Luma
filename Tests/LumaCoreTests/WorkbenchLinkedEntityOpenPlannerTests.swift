import Foundation
import Testing
import LumaCore

@Test func linkedEntityPlannerDispatchesTodoCaptureToReplaceQuery() {
    let entry = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .workbenchTodo,
        title: "Ship feature",
        preview: "Ship feature",
        resumePayloadJSON: WorkbenchActivityResumePayload.todoCapture("Ship feature").encoded()
    )
    let action = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry)
    guard case .replaceQuery(let query) = action else {
        Issue.record("Expected replaceQuery")
        return
    }
    #expect(query.contains("Ship feature"))
}

@Test func linkedEntityPlannerDispatchesSnippetDraftToResume() {
    let entryID = UUID()
    let entry = WorkbenchActivityEntry(
        id: entryID,
        kind: .projectLinked,
        moduleID: .workbenchSnippets,
        title: "Draft snippet",
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let action = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry)
    guard case .resumeActivity(let id) = action else {
        Issue.record("Expected resumeActivity")
        return
    }
    #expect(id == entryID)
}

@Test func linkedEntityPlannerUsesEntityRefWhenNoActivityEntry() {
    let link = WorkbenchProjectLink(
        stableProjectID: "path:test",
        entityRef: WorkbenchEntityRef(
            kind: .quicklink,
            entityID: UUID().uuidString,
            moduleID: .workbenchQuicklinks,
            title: "Docs"
        )
    )
    let action = WorkbenchLinkedEntityOpenPlanner.rowAction(for: link, entry: nil)
    guard case .openModule(let moduleID) = action else {
        Issue.record("Expected openModule")
        return
    }
    #expect(moduleID == .workbenchQuicklinks)
}
