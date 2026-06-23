import Foundation
import LumaCore
import Testing
@testable import LumaServices
@testable import LumaModules

@Test func clipboardModuleRequiresClipPrefixInRootSearch() async {
    let module = ClipboardModule()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(30)))

    let appOnlySearch = await module.handle(Query(raw: "cursor", sequence: 1), context: context)
    #expect(appOnlySearch.items.isEmpty)

    let clipHelp = await module.handle(Query(raw: "clip ?", sequence: 2), context: context)
    #expect(!clipHelp.items.isEmpty)
}

@Test func clipboardModuleImageResultUsesCustomCopyAction() async throws {
    let store = ClipboardHistoryStore()
    let imageData = Data([0x89, 0x50, 0x4E, 0x47])
    await store.add(
        text: "[Image]",
        types: ["public.png"],
        imageData: imageData,
        imagePasteboardType: "public.png"
    )
    let entryID = await store.search("").first!.id
    let module = ClipboardModule(store: store, persistenceURL: URL(fileURLWithPath: "/tmp/luma-clipboard-test-\(UUID().uuidString).json"))
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(30)))
    let results = await module.handle(Query(raw: "clip", sequence: 1), context: context)
    #expect(results.items.count == 1)
    #expect(results.items.first?.title.contains("Image") == true)
    if case .custom(_, let handler) = results.items.first!.primaryAction.kind {
        #expect(handler == .clipboard)
    } else {
        Issue.record("Expected custom clipboard copy action")
    }

    let pasteboard = RecordingPasteboardClient()
    let actionContext = ActionContext(
        logger: NoopLogger(),
        metrics: NoopMetricsClient(),
        pasteboard: pasteboard,
        accessibility: NoopAccessibilityClient()
    )
    try await module.perform(results.items.first!.primaryAction, context: actionContext)
    let recorded = await pasteboard.snapshot()
    #expect(recorded.imageData == imageData)
    #expect(recorded.imageType == "public.png")
    #expect(recorded.text == nil)
    _ = entryID
}

private actor RecordingPasteboardClient: PasteboardClient {
    private var text: String?
    private var imageData: Data?
    private var imageType: String?

    func write(_ string: String) async {
        text = string
        imageData = nil
        imageType = nil
    }

    func writeSecure(_ string: String, clearAfterSeconds: Int) async {
        await write(string)
    }

    func writeImage(data: Data, pasteboardType: String) async {
        imageData = data
        imageType = pasteboardType
        text = nil
    }

    func snapshot() -> (text: String?, imageData: Data?, imageType: String?) {
        (text, imageData, imageType)
    }
}

private struct NoopLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private struct NoopAccessibilityClient: AccessibilityClient {
    func focus(windowID: UInt32, pid: Int32, title: String) async {}
    func insert(text: String) async {}
    func applyWindowLayout(_ preset: String) async {}
}

@Test func clipboardFilterBlocksSecrets() {
    #expect(ClipboardFilter.shouldSkip(types: ["org.nspasteboard.ConcealedType"]))
    #expect(ClipboardFilter.shouldSkip(types: ["com.1password.item"]))
    #expect(ClipboardFilter.shouldSkip(types: ["public.password"]))
}

@Test func clipboardHistoryStoresSearchesPinsAndClears() async {
    let store = ClipboardHistoryStore(maxEntries: 3)
    await store.add(text: "hello world", types: ["public.utf8-plain-text"])
    await store.add(text: "api token", types: ["public.password"])
    await store.add(text: "raycast style", types: ["public.utf8-plain-text"])

    let results = await store.search("ray")
    #expect(results.count == 1)
    #expect(results.first?.text == "raycast style")

    if let first = await store.search("").first {
        await store.pin(first.id)
    }
    #expect(await store.search("").first?.isPinned == true)

    await store.clear()
    #expect(await store.search("").isEmpty)
}

@Test func clipboardHistoryPrunesByCount() async {
    let store = ClipboardHistoryStore(maxEntries: 2)
    await store.add(text: "one", types: ["public.text"])
    await store.add(text: "two", types: ["public.text"])
    await store.add(text: "three", types: ["public.text"])
    let results = await store.search("")
    #expect(results.map(\.text) == ["three", "two"])
}

@Test func clipboardHistoryTTLPrunesExpiredUnpinnedButKeepsPinned() async {
    let now = Date()
    let store = ClipboardHistoryStore(maxAge: 10_000)
    await store.add(text: "expired", types: ["public.text"], now: now.addingTimeInterval(-120))
    await store.add(text: "pinned expired", types: ["public.text"], now: now.addingTimeInterval(-120))
    await store.add(text: "fresh", types: ["public.text"], now: now)

    let pinnedID = await store.search("pinned").first!.id
    await store.pin(pinnedID)
    await store.updateRetention(maxEntries: 500, maxAge: 60, maxTextBytes: 100 * 1024, now: now)

    let results = await store.search("")
    #expect(results.map(\.text) == ["pinned expired", "fresh"])
}

@Test func clipboardHistoryRejectsOversizedText() async {
    let store = ClipboardHistoryStore()
    let huge = String(repeating: "x", count: 101 * 1024)
    await store.add(text: huge, types: ["public.text"])
    #expect(await store.search("").isEmpty)
}

@Test func clipboardHistoryUsesConfiguredTextLimit() async {
    let store = ClipboardHistoryStore(maxTextBytes: 8)
    await store.add(text: "small", types: ["public.text"])
    await store.add(text: "too-large", types: ["public.text"])
    #expect(await store.search("").map(\.text) == ["small"])
}

@Test func clipboardHistoryMovesDuplicateToTop() async {
    let store = ClipboardHistoryStore()
    await store.add(text: "first", types: ["public.text"])
    await store.add(text: "second", types: ["public.text"])
    await store.add(text: "first", types: ["public.text"])
    #expect(await store.search("").map(\.text).prefix(2) == ["first", "second"])
}

@Test func clipboardPinnedEntriesSortFirst() async {
    let store = ClipboardHistoryStore()
    await store.add(text: "older", types: ["public.text"])
    await store.add(text: "newer", types: ["public.text"])
    let olderID = await store.search("older").first!.id
    await store.pin(olderID)
    let ordered = await store.search("")
    #expect(ordered.first?.text == "older")
    #expect(ordered.first?.isPinned == true)
}

@Test func clipboardFilterImageOnly() async {
    let store = ClipboardHistoryStore()
    await store.add(text: "plain note", types: ["public.text"])
    await store.add(
        text: "[Image]",
        types: ["public.png"],
        imageData: Data([0x89, 0x50, 0x4E, 0x47]),
        imagePasteboardType: "public.png"
    )
    await store.add(text: "another note", types: ["public.text"])
    let images = await store.list(filter: .image, query: "", limit: 10)
    #expect(images.count == 1)
    #expect(images.first?.detectedKind == .image)
}

@Test func clipboardDetectsImageFromPasteboardTypes() {
    #expect(ClipboardEntryKind.detect(from: "[Image]", pasteboardTypes: ["public.png"]) == .image)
    #expect(ClipboardEntryKind.isImageTypes(["public.tiff"]))
}

@Test func clipboardTokenSearchMatchesAllTerms() async {
    let store = ClipboardHistoryStore()
    await store.add(text: "hello brave world", types: ["public.text"])
    await store.add(text: "hello world", types: ["public.text"])
    let results = await store.search("hello brave")
    #expect(results.count == 1)
    #expect(results.first?.text == "hello brave world")
}

@Test func clipboardPinAndDeletePersist() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("clipboard-pin-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = ClipboardHistoryStore(persistenceURL: url)
    await store.add(text: "keep me", types: ["public.text"])
    let id = await store.search("keep").first!.id
    await store.pin(id)

    let reloaded = ClipboardHistoryStore(persistenceURL: url)
    #expect(await reloaded.search("keep").first?.isPinned == true)

    await reloaded.removeEntry(id)
    let again = ClipboardHistoryStore(persistenceURL: url)
    #expect(await again.search("keep").isEmpty)
}

@Test func clipboardDetectsEntryKinds() {
    #expect(ClipboardEntryKind.detect(from: "https://example.com") == .link)
    #expect(ClipboardEntryKind.detect(from: "user@example.com") == .email)
    #expect(ClipboardEntryKind.detect(from: "func main() {\n}\n") == .code)
    #expect(ClipboardEntryKind.detect(from: "plain text") == .text)
}

@Test func translationEmptyInputSkipsRequest() {
    #expect(TranslationUserMessages.shouldTranslate("") == false)
    #expect(TranslationUserMessages.shouldTranslate("   ") == false)
    #expect(TranslationUserMessages.shouldTranslate("hello") == true)
}

@Test func translationFailureProducesUserFacingMessage() {
    let message = TranslationUserMessages.message(for: SystemTranslationError.shortcutUnavailable)
    #expect(message.contains("Luma Translate"))
}

@Test func translationSystemErrorsDefineShortcutFallbackPolicy() {
    #expect(SystemTranslationError.frameworkUnavailable.allowsShortcutFallback)
    #expect(SystemTranslationError.emptyOutput.allowsShortcutFallback)
    #expect(SystemTranslationError.languagePackRequired.allowsShortcutFallback == false)
    #expect(SystemTranslationError.shortcutTimedOut.allowsShortcutFallback == false)
}

@Test func translateModuleResultKeyDoesNotPersistRawText() {
    let text = "sensitive user phrase"
    let key = TranslateModule.resultKey(for: text)
    #expect(key != text)
    #expect(key.count == 16)
    #expect(TranslateModule.resultKey(for: text) == key)
}

@Test func translationLanguageDetectorRecognizesChinese() {
    let code = TranslationLanguageDetector.detectedLanguageCode(for: "你好")
    #expect(code == "zh-Hans" || code == "zh-Hant")
}
