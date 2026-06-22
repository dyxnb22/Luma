import Foundation
import Testing
@testable import LumaModules

@Test func mediaStoreRoundTrips() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-test-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    let draft = MediaEditorDraft(title: "Oppenheimer", category: .movie, status: .done, rating: 9)
    let created = try await store.add(from: draft)
    #expect(created.title == "Oppenheimer")

    let reloaded = MediaStore(persistenceURL: url)
    let items = await reloaded.all()
    #expect(items.count == 1)
    #expect(items[0].rating == 9)
}

@Test func mediaStoreUpdateAndDelete() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-upd-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    var draft = MediaEditorDraft(title: "Dune", category: .book, status: .inProgress)
    let item = try await store.add(from: draft)
    draft = MediaEditorDraft(item: item)
    draft.status = .done
    draft.rating = 8
    _ = try await store.update(from: draft)
    let updated = await store.all()
    #expect(updated[0].status == .done)
    #expect(updated[0].rating == 8)

    try await store.delete(id: item.id)
    #expect(await store.all().isEmpty)
}
