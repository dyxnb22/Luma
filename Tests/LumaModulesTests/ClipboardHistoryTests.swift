import Foundation
import Testing
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

@Test func clipboardHistoryMovesDuplicateToTop() async {
    let store = ClipboardHistoryStore()
    await store.add(text: "first", types: ["public.text"])
    await store.add(text: "second", types: ["public.text"])
    await store.add(text: "first", types: ["public.text"])
    #expect(await store.search("").map(\.text).prefix(2) == ["first", "second"])
}
