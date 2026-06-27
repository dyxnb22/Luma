import Foundation
import LumaCore
import LumaInfrastructure
import LumaServices
import Testing
@testable import LumaModules

@Test func mediaModuleExtractPayload() {
    #expect(MediaModule.extractPayload(raw: "rec") == "")
    #expect(MediaModule.extractPayload(raw: "record") == "")
    #expect(MediaModule.extractPayload(raw: "log") == "")
    #expect(MediaModule.extractPayload(raw: "m") == "")
    #expect(MediaModule.extractPayload(raw: "media") == "")
    #expect(MediaModule.extractPayload(raw: "rec 三体 book done 9") == "三体 book done 9")
    #expect(MediaModule.extractPayload(raw: "record luma book 8") == "luma book 8")
    #expect(MediaModule.extractPayload(raw: "log 三体") == "三体")
    #expect(MediaModule.extractPayload(raw: "m old syntax movie 7") == "old syntax movie 7")
    #expect(MediaModule.extractPayload(raw: "login") == nil)
    #expect(MediaModule.extractPayload(raw: "translate hello") == nil)
}

@Test func mediaModuleRecReturnsRecentOnly() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-mod-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    _ = try await store.add(from: MediaEditorDraft(title: "Dune", category: .book, status: .done, rating: 8))
    let module = MediaModule(store: store)
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "rec", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Dune")
    #expect(!result.items.contains { $0.subtitle == "Open full logbook" })
}

@Test func mediaModuleRecLogOpensDetail() async {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-log-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "rec log", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Records")
    #expect(result.items.first?.primaryAction.title == "Open Records")
}

@Test func mediaModuleRecordCaptureRow() async {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-cap-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "record luma book 8", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Log luma")
    #expect(result.items.first?.subtitle?.contains("Book") == true)
    #expect(result.items.first?.subtitle?.contains("★8") == true)
    #expect(result.items.first?.primaryAction.title == "Log Item")
}

@Test func mediaModuleLogSearchesExistingItem() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-srch-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    _ = try await store.add(from: MediaEditorDraft(title: "三体", category: .book, status: .done, rating: 9, tags: ["sci-fi"]))
    let module = MediaModule(store: store)
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "log 三体", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "三体")
    #expect(result.items.first?.primaryAction.title == "Edit Item")
}

@Test func mediaModuleLegacyMSyntaxStillWorks() async {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-legacy-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "m old syntax movie 7", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Log old syntax")
    #expect(result.items.first?.primaryAction.title == "Log Item")
}

@Test func mediaModuleRecCaptureWithTags() async {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-tags-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "rec 三体 book done 9 #sci-fi", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Log 三体")
    #expect(result.items.first?.subtitle?.contains("#sci-fi") == true)
    #expect(result.items.first?.primaryAction.title == "Log Item")
}

@Test func mediaModulePartialCaptureWithoutCategory() async {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-part-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "rec The Bear", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Complete Entry")
    #expect(result.items.first?.subtitle?.contains("needs category") == true)
    #expect(result.items.first?.primaryAction.title == "Complete Entry")
}

@Test func mediaModulePartialCaptureKeepsTagsOutOfTitle() async {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-part-tag-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(testMediaModuleContext())

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "rec Dune #sci-fi", sequence: 1), context: context)

    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Complete Entry")
    #expect(result.items.first?.subtitle?.contains("#sci-fi") == true)
    if case .openModuleDetail(let module, let payload) = result.items.first?.primaryAction.kind,
       module.rawValue == "luma.media",
       let payload,
       let action = try? ModuleActionCoding.decode(MediaAction.self, from: payload),
       case .editDraft(let draft) = action {
        #expect(draft.title == "Dune")
        #expect(draft.tags == ["sci-fi"])
    } else {
        Issue.record("Expected edit draft action")
    }
}

@Test func mediaModuleDefaultEnabled() {
    #expect(MediaModule.manifest.defaultEnabled == false)
    #expect(MediaModule.manifest.displayName == "Records")
}

@Test func mediaModuleInProgressCount() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-progress-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    _ = try await store.add(from: MediaEditorDraft(title: "Frieren", category: .anime, status: .inProgress))
    _ = try await store.add(from: MediaEditorDraft(title: "Dune", category: .book, status: .done, rating: 9))
    _ = try await store.add(from: MediaEditorDraft(title: "Elden Ring", category: .game, status: .inProgress))

    let module = MediaModule(store: store)
    await module.warmup(testMediaModuleContext())
    #expect(await module.inProgressCount() == 2)
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
