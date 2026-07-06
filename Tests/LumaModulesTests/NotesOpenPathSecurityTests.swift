import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private struct NotesSecurityTestLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

@Test func notesOpenRejectsPathOutsideRoot() async throws {
    let temp = FileManager.default.temporaryDirectory
    let root = temp.appendingPathComponent("notes-root-\(UUID().uuidString)", isDirectory: true)
    let outside = temp.appendingPathComponent("outside-\(UUID().uuidString).md")
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "test".write(to: outside, atomically: true, encoding: .utf8)

    let configURL = temp.appendingPathComponent("notes-\(UUID().uuidString).json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    var config = NotesRootConfig.empty
    config.root = root
    try await configStore.save(config)

    let module = NotesModule(index: NotesTreeIndex(), config: configStore)
    await module.warmup(ModuleContext(
        logger: NotesSecurityTestLogger(),
        metrics: NoopMetricsClient(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))

    let payload = try ModuleActionCoding.encode(NotesAction.open(path: outside.path))
    let action = Action(
        id: ActionID(module: .notes, key: "open"),
        title: "Open",
        kind: .custom(payload: payload, handler: .notes)
    )
    let context = ActionContext(
        logger: NotesSecurityTestLogger(),
        metrics: NoopMetricsClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        translation: NoopTranslationClient(),
        workspace: NoopWorkspaceClient()
    )

    do {
        try await module.perform(action, context: context)
        Issue.record("Expected pathOutsideRoot error")
    } catch let error as NotesActionError {
        #expect(error == .pathOutsideRoot)
    }
}
