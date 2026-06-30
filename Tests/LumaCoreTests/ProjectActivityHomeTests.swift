import Foundation
import Testing
import LumaCore

@Test func workbenchCommandRouterMatchesProjectCommands() {
    let router = WorkbenchCommandRouter()
    #expect(router.route(raw: "proj work") == .projectWork)
    #expect(router.route(raw: "proj recent") == .projectRecent)
    #expect(router.route(raw: "attach clip") == .attachClipboard)
    #expect(router.route(raw: "attach sel") == .attachSelection)
    #expect(router.route(raw: "proj note") != .none)
    #expect(router.route(raw: "proj todo") != .none)
}

@Test func workbenchCommandPreviewRowsHaveNoSideEffects() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("preview-store-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = WorkbenchActivityStore(fileURL: url)
    let before = await store.allEntries().count
    let context = WorkbenchContext(
        clipboardPreview: "https://example.com",
        enabledModuleIDs: [.workbenchQuicklinks],
        pinnedModuleIDs: [.workbenchQuicklinks]
    )
    let rows = WorkbenchCommandResults.previewRows(
        route: .capture(WorkbenchCommandRouter().definition(for: .captureClipboardQuicklink)!),
        querySequence: 1,
        context: context
    )
    let after = await store.allEntries().count
    #expect(!rows.isEmpty)
    #expect(before == after)
}

@Test func projectActivityDraftQueryRespectsEnabledModules() {
    let path = "/tmp/luma"
    let activity = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Project snippet",
        project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let drafts = WorkbenchActivityQuery.recentDrafts(forProject: path, entries: [activity])
    #expect(drafts.count == 1)
    let enabled: Set<ModuleIdentifier> = [.projects]
    let gated = drafts.filter { enabled.contains($0.moduleID) }
    #expect(gated.isEmpty)
}

@Test func projectActivityDraftQueryIncludesEnabledModuleDrafts() {
    let path = "/tmp/luma"
    let activity = WorkbenchActivityEntry(
        kind: .projectLinked,
        moduleID: .snippets,
        title: "Project snippet",
        project: WorkbenchProjectAssociation(projectPath: path, projectLabel: "Luma"),
        resumeRef: WorkbenchResumeRef(kind: .snippetDraft, entryID: UUID()),
        resumePayloadJSON: WorkbenchActivityResumePayload.snippetDraft(Data("{}".utf8)).encoded()
    )
    let drafts = WorkbenchActivityQuery.recentDrafts(forProject: path, entries: [activity])
    let enabled: Set<ModuleIdentifier> = [.projects, .snippets]
    let gated = drafts.filter { enabled.contains($0.moduleID) }
    #expect(gated.count == 1)
}
