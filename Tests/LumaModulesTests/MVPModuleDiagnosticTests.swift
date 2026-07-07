import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing
@testable import LumaModules

// MARK: - Apps

@Test func appsTopColdHandleSurfacesMemoryWarmupDiagnostic() async {
    let module = AppsModule()
    let parsed = ParsedCommand(trigger: "app", payload: "top", module: .apps)
    let query = Query(raw: "app top", sequence: 1, command: parsed)
    let context = QueryContext(deadline: .now + .seconds(1))

    let result = await module.handle(query, context: context)

    #expect(result.items.isEmpty)
    #expect(result.diagnostic?.kind == .degraded)
    #expect(result.diagnostic?.message == "Memory usage cache warming")
}

// MARK: - Notes

@Test func notesBareQueryWithoutRootSurfacesOnboardingRow() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-notes-diagnostic-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let configURL = tempDir.appendingPathComponent("notes-root.json")
    let configStore = NotesRootConfigStore(fileURL: configURL)
    let module = NotesModule(index: NotesTreeIndex(), config: configStore)
    await module.warmup(ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))

    let parsed = ParsedCommand(trigger: "n", payload: "", module: .notes)
    let query = Query(raw: "n", sequence: 1, command: parsed)
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1)))

    #expect(result.items.count == 1)
    #expect(result.items[0].id.key == "no-root")
    #expect(result.items[0].title == "Choose a Notes root folder")
    await module.teardown()
}

// MARK: - Clipboard

@Test func clipboardPasteReturnsPermissionRequiredWhenAXDenied() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-clipboard-diagnostic-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let storeURL = tempDir.appendingPathComponent("clipboard-history.json")
    let store = ClipboardHistoryStore(persistenceURL: storeURL)
    await store.add(text: "hello", types: ["public.text"])
    let id = await store.search("").first!.id

    let module = ClipboardModule(
        store: store,
        persistenceURL: storeURL,
        pasteboard: RecordingPasteboardClient(),
        accessibility: DeniedAccessibilityClient()
    )
    await module.warmup(ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: DeniedAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))

    let outcome = try await module.pasteEntry(id: id)
    #expect(outcome == .permissionRequired)
    await module.teardown()
}

private struct DeniedAccessibilityClient: AccessibilityClient {
    func isTrusted() async -> Bool { false }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { false }
    func applyWindowLayout(_ preset: String) async {}
}

private actor RecordingPasteboardClient: PasteboardClient {
    func write(_ string: String) async {}
    func writeSecure(_ string: String, clearAfterSeconds: Int) async {}
    func writeImage(data: Data, pasteboardType: String) async {}
    func writeFileURLs(_ urls: [URL]) async {}
    func readString() async -> String? { nil }
}
