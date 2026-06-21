import Foundation
import Testing
@testable import LumaModules

@Test func clipboardHistoryPersistsAcrossStores() async {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("clipboard.json")

    let first = ClipboardHistoryStore(persistenceURL: url)
    await first.add(text: "persist me", types: ["public.text"])

    let second = ClipboardHistoryStore(persistenceURL: url)
    #expect(await second.search("persist").first?.text == "persist me")
}
