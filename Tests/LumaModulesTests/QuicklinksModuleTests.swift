import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private final class PlatformReadCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var projectReads = 0
    private var selectionReads = 0
    private var clipboardReads = 0

    func recordProject() {
        lock.lock()
        projectReads += 1
        lock.unlock()
    }

    func recordSelection() {
        lock.lock()
        selectionReads += 1
        lock.unlock()
    }

    func recordClipboard() {
        lock.lock()
        clipboardReads += 1
        lock.unlock()
    }

    func snapshot() -> (project: Int, selection: Int, clipboard: Int) {
        lock.lock()
        defer { lock.unlock() }
        return (projectReads, selectionReads, clipboardReads)
    }
}

private struct CountingCurrentProjectClient: CurrentProjectClient {
    let counter: PlatformReadCounter

    func snapshot() async -> CurrentProjectContext? {
        counter.recordProject()
        return nil
    }
}

private struct CountingSelectionClient: SelectionSnapshotClient {
    let counter: PlatformReadCounter

    func snapshot() async -> String? {
        counter.recordSelection()
        return nil
    }
}

private struct CountingPasteboardClient: PasteboardClient {
    let counter: PlatformReadCounter

    func write(_ string: String) async {}
    func writeSecure(_ string: String, clearAfterSeconds: Int) async {}
    func writeImage(data: Data, pasteboardType: String) async {}
    func writeFileURLs(_ urls: [URL]) async {}
    func readString() async -> String? {
        counter.recordClipboard()
        return nil
    }
}

@Test func quicklinksHandleDoesNotReadPlatformState() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-quicklinks-test-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let storeURL = tempDir.appendingPathComponent("quicklinks.json")
    let store = QuicklinksStore(url: storeURL)
    let quicklink = Quicklink(
        name: "GitHub Search",
        trigger: "gh",
        urlTemplate: "https://github.com/search?q={{query}}",
        icon: "link"
    )
    _ = try await store.add(quicklink)

    let counter = PlatformReadCounter()
    let module = QuicklinksModule(store: store)
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

    let query = Query(raw: "gh swift", sequence: 1)
    let platform = QueryPlatformClients(
        pasteboard: CountingPasteboardClient(counter: counter),
        currentProject: CountingCurrentProjectClient(counter: counter),
        selectionSnapshot: CountingSelectionClient(counter: counter)
    )
    let result = await module.handle(
        query,
        context: QueryContext(deadline: .now + .seconds(1), platform: platform)
    )

    #expect(!result.items.isEmpty)
    let counts = counter.snapshot()
    #expect(counts.project == 0)
    #expect(counts.selection == 0)
    #expect(counts.clipboard == 0)
}
