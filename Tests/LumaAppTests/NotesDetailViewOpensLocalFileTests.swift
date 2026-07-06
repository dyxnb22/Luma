import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

/// Mirrors NotesDetailView Today chip + linked-notes open paths (local file URLs only).
@Test func notesDetailDailyOpenUsesLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-detail-daily-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = NotesModule(index: NotesTreeIndex(), config: configStore)
    await module.warmup(ModuleContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))

    let loaded = await module.loadConfig()
    let url = try await NotesActions(index: await module.treeIndex()).openOrCreateDailyNote(
        root: loaded.root!,
        dailyFolderName: loaded.dailyFolderName
    )

    let workspace = DetailViewRecordingWorkspaceClient()
    try await workspace.openLocalFileURL(url)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal.count == 1)
    #expect(calls.openLocal[0].isFileURL)
}

@Test func notesDetailLinkedNoteSelectionUsesLocalFileURL() async throws {
    let temp = FileManager.default.temporaryDirectory
    let note = temp.appendingPathComponent("linked-\(UUID().uuidString).md")
    try "# Linked".write(to: note, atomically: true, encoding: .utf8)

    let workspace = DetailViewRecordingWorkspaceClient()
    try await workspace.openLocalFileURL(note)

    let calls = workspace.snapshot()
    #expect(calls.openURL.isEmpty)
    #expect(calls.openLocal == [note.standardizedFileURL])
}

private struct NoopLoggingClient: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private final class DetailViewRecordingWorkspaceClient: WorkspaceClient, @unchecked Sendable {
    private let lock = NSLock()
    private(set) var openURLCalls: [URL] = []
    private(set) var openLocalFileURLCalls: [URL] = []

    func launchApplication(at url: URL) async throws { _ = url }
    func openURL(_ url: URL) async throws {
        lock.withLock { openURLCalls.append(url) }
        try ExternalURLPolicy.validateOpenURL(url, allowFileURLs: false)
    }
    func openLocalFileURL(_ url: URL) async throws {
        lock.withLock { openLocalFileURLCalls.append(url.standardizedFileURL) }
    }
    func revealInFinder(_ url: URL) async throws { _ = url }
    func terminateApplication(bundleID: String) async { _ = bundleID }
    func openApplication(bundleID: String, arguments: [String]) async { _ = bundleID; _ = arguments }

    func snapshot() -> (openURL: [URL], openLocal: [URL]) {
        lock.withLock { (openURLCalls, openLocalFileURLCalls) }
    }
}
