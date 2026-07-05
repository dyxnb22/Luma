import Foundation
import Testing
import LumaCore

@Test func workbenchActionBridgeOpensModuleDetail() {
    let workbench = WorkbenchAction(
        kind: .prepareDraft,
        targetModule: .workbenchSnippets,
        title: "Open Snippets",
        payload: Data([1, 2, 3])
    )
    let action = WorkbenchActionBridge.launcherAction(for: workbench, actionKey: "test")
    guard case .openModuleDetail(let module, let payload) = action.kind else {
        Issue.record("Expected openModuleDetail")
        return
    }
    #expect(module == .workbenchSnippets)
    #expect(payload == Data([1, 2, 3]))
}

@Test func workbenchActivityStoreRecordsCapture() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("workbench-activity-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }
    let store = WorkbenchActivityStore(fileURL: url)
    _ = await store.recordCapture(
        target: .quicklinkDraft,
        moduleID: .workbenchQuicklinks,
        preview: "github"
    )
    let snapshot = await store.snapshot(limit: 1)
    #expect(snapshot.count == 1)
    #expect(snapshot[0].kind == .draftPrepared)
    #expect(snapshot[0].entityKind == .quicklink)
}

@Test func workbenchContextEnablementHelpers() {
    let context = WorkbenchContext(
        enabledModuleIDs: [.workbenchNotes],
        pinnedModuleIDs: [.workbenchNotes]
    )
    #expect(context.isEnabled(.workbenchNotes))
    #expect(!context.isEnabled(.workbenchTodo))
    #expect(context.isHot(.workbenchNotes))
    #expect(!context.isHot(.workbenchTodo))
}
