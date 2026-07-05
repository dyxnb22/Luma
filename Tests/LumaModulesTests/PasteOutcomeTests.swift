import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private struct DeniedAccessibilityClient: AccessibilityClient {
    func isTrusted() async -> Bool { false }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { false }
    func applyWindowLayout(_ preset: String) async {}
}

private actor RecordingPasteboardClient: PasteboardClient {
    private(set) var lastWrite: String?

    func write(_ string: String) async { lastWrite = string }
    func writeSecure(_ string: String, clearAfterSeconds: Int) async { lastWrite = string }
    func writeImage(data: Data, pasteboardType: String) async {}
    func writeFileURLs(_ urls: [URL]) async {}
    func readString() async -> String? { nil }

    func lastWritten() -> String? { lastWrite }
}

private func snippetsContext(
    pasteboard: any PasteboardClient,
    accessibility: any AccessibilityClient
) -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: pasteboard,
        accessibility: accessibility,
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    )
}

@Test func snippetsInsertReturnsPermissionRequiredWhenAXDenied() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-snippets-test-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let storeURL = tempDir.appendingPathComponent("snippets.json")
    let store = SnippetsStore(persistenceURL: storeURL)
    let snippet = try await store.add(title: "Gitignore", content: "node_modules", tags: [], trigger: "gitignore")

    let pasteboard = RecordingPasteboardClient()
    let module = SnippetsModule(store: store)
    await module.warmup(snippetsContext(pasteboard: pasteboard, accessibility: DeniedAccessibilityClient()))

    let outcome = try await module.insertSnippet(id: snippet.id)
    #expect(outcome == .permissionRequired)
    #expect(await pasteboard.lastWritten() == "node_modules")
}

@Test func snippetsInsertReturnsPastedWhenAXTrusted() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-snippets-test-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let storeURL = tempDir.appendingPathComponent("snippets.json")
    let store = SnippetsStore(persistenceURL: storeURL)
    let snippet = try await store.add(title: "Hello", content: "Hello", tags: [], trigger: "hello")

    let module = SnippetsModule(store: store)
    await module.warmup(snippetsContext(
        pasteboard: RecordingPasteboardClient(),
        accessibility: TrustedAccessibilityClient()
    ))

    let outcome = try await module.insertSnippet(id: snippet.id)
    #expect(outcome == .pasted)
}

private struct TrustedAccessibilityClient: AccessibilityClient {
    func isTrusted() async -> Bool { true }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { false }
    func applyWindowLayout(_ preset: String) async {}
}
