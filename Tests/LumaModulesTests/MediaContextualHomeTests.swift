import Foundation
import LumaCore
import LumaInfrastructure
import LumaServices
import Testing
@testable import LumaModules

@Test func mediaInProgressCountDrivesContinueRecordsSubtitle() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-home-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    _ = try await store.add(from: MediaEditorDraft(title: "Frieren", category: .anime, status: .inProgress))
    _ = try await store.add(from: MediaEditorDraft(title: "The Bear", category: .tv, status: .inProgress))
    _ = try await store.add(from: MediaEditorDraft(title: "Dune", category: .book, status: .done))

    let module = MediaModule(store: store)
    await module.warmup(testMediaModuleContext())

    let count = await module.inProgressCount()
    #expect(count == 2)
    let subtitle = count == 1 ? "1 in progress" : "\(count) in progress"
    #expect(subtitle == "2 in progress")
}

@Test func mediaInProgressCountIsZeroWhenAllDone() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-home-empty-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    _ = try await store.add(from: MediaEditorDraft(title: "Dune", category: .book, status: .done))

    let module = MediaModule(store: store)
    await module.warmup(testMediaModuleContext())
    #expect(await module.inProgressCount() == 0)
}

private func testMediaModuleContext() -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(),
        config: ConfigurationStore()
    )
}
