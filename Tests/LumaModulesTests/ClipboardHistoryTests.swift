import Foundation
import Testing
@testable import LumaServices
@testable import LumaModules

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

@Test func clipboardFilterLinksOnly() async {
    let store = ClipboardHistoryStore()
    await store.add(text: "plain note", types: ["public.text"])
    await store.add(text: "https://luma.app", types: ["public.text"])
    await store.add(text: "user@example.com", types: ["public.text"])
    let links = await store.list(filter: .links, query: "", limit: 10)
    #expect(links.count == 2)
    #expect(links.allSatisfy { $0.detectedKind == .link || $0.detectedKind == .email })
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
