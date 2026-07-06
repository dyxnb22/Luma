import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private struct NotesCreateTestLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private final class RecordingWorkspaceClient: WorkspaceClient, @unchecked Sendable {
    private let lock = NSLock()
    private(set) var openURLCalls: [URL] = []
    private(set) var openLocalFileURLCalls: [URL] = []

    func launchApplication(at url: URL) async throws {
        _ = url
    }

    func openURL(_ url: URL) async throws {
        lock.withLock { openURLCalls.append(url) }
        try ExternalURLPolicy.validateOpenURL(url, allowFileURLs: false)
    }

    func openLocalFileURL(_ url: URL) async throws {
        lock.withLock { openLocalFileURLCalls.append(url) }
    }

    func revealInFinder(_ url: URL) async throws {
        _ = url
    }

    func terminateApplication(bundleID: String) async {
        _ = bundleID
    }

    func openApplication(bundleID: String, arguments: [String]) async {
        _ = bundleID
        _ = arguments
    }

    func snapshot() -> (openURL: [URL], openLocal: [URL]) {
        lock.withLock { (openURLCalls, openLocalFileURLCalls) }
    }
}

private func makeNotesModule(root: URL, configStore: NotesRootConfigStore) async -> NotesModule {
    let module = NotesModule(index: NotesTreeIndex(), config: configStore)
    await module.warmup(ModuleContext(
        logger: NotesCreateTestLogger(),
        metrics: NoopMetricsClient(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))
    return module
}

private func performNotesAction(
    _ notesAction: NotesAction,
    module: NotesModule,
    workspace: RecordingWorkspaceClient
) async throws {
    let payload = try ModuleActionCoding.encode(notesAction)
    let action = Action(
        id: ActionID(module: .notes, key: "test"),
        title: "Test",
        kind: .custom(payload: payload, handler: .notes)
    )
    let context = ActionContext(
        logger: NotesCreateTestLogger(),
        metrics: NoopMetricsClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        translation: NoopTranslationClient(),
        workspace: workspace
    )
    try await module.perform(action, context: context)
}

@Test func notesCreateInInboxOpensViaLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-create-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = await makeNotesModule(root: root, configStore: configStore)
    let workspace = RecordingWorkspaceClient()
    try await performNotesAction(.createInInbox(title: "Review"), module: module, workspace: workspace)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal.count == 1)
    #expect(calls.openLocal[0].isFileURL)
}

@Test func notesOpenOrCreateDailyOpensViaLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-daily-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = await makeNotesModule(root: root, configStore: configStore)
    let workspace = RecordingWorkspaceClient()
    try await performNotesAction(.openOrCreateDaily, module: module, workspace: workspace)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal.count == 1)
    #expect(calls.openLocal[0].isFileURL)
}

@Test func notesCaptureToDailyOpensViaLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-capture-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = await makeNotesModule(root: root, configStore: configStore)
    let workspace = RecordingWorkspaceClient()
    try await performNotesAction(.captureToDaily(text: "Captured line"), module: module, workspace: workspace)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal.count == 1)
}

@Test func notesCreateWeeklyReviewOpensViaLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-weekly-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = await makeNotesModule(root: root, configStore: configStore)
    let workspace = RecordingWorkspaceClient()
    try await performNotesAction(.createWeeklyReview, module: module, workspace: workspace)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal.count == 1)
}

@Test func notesCreateFromTemplateOpensViaLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-template-\(UUID().uuidString)", isDirectory: true)
    let templatesDir = root.appendingPathComponent("_templates", isDirectory: true)
    try FileManager.default.createDirectory(at: templatesDir, withIntermediateDirectories: true)
    let templateURL = templatesDir.appendingPathComponent("Meeting.md")
    try "# {{title}}\n".write(to: templateURL, atomically: true, encoding: .utf8)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = await makeNotesModule(root: root, configStore: configStore)
    let workspace = RecordingWorkspaceClient()
    try await performNotesAction(.createFromTemplate(template: "meeting", title: "Standup"), module: module, workspace: workspace)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal.count == 1)
    #expect(calls.openLocal[0].isFileURL)
}
