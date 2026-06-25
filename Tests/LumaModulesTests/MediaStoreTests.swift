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

@Test func mediaStoreNormalizesTags() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-tags-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    let draft = MediaEditorDraft(title: "三体", category: .book, status: .done, tags: ["Sci-Fi", " Favorite "])
    let created = try await store.add(from: draft)
    #expect(created.tags == ["sci-fi", "favorite"])
}

@Test func mediaStoreCapsNotes() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-notes-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: url)
    let longNotes = String(repeating: "a", count: 2500)
    let draft = MediaEditorDraft(title: "Note", category: .book, status: .done, notes: longNotes)
    let created = try await store.add(from: draft)
    #expect(created.notes.count == 2000)
}

@Test func mediaModuleExportCSVIncludesTags() async throws {
    let storeURL = FileManager.default.temporaryDirectory.appendingPathComponent("media-export-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: storeURL.deletingLastPathComponent()) }

    let store = MediaStore(persistenceURL: storeURL)
    _ = try await store.add(from: MediaEditorDraft(
        title: "三体",
        category: .book,
        status: .done,
        rating: 9,
        tags: ["sci-fi"]
    ))

    let module = MediaModule(store: store)
    let url = try await module.exportCSV()
    defer { try? FileManager.default.removeItem(at: url) }

    #expect(url.lastPathComponent.hasPrefix("luma-records-"))
    #expect(url.pathExtension == "csv")
    let csv = try String(contentsOf: url, encoding: .utf8)
    #expect(csv.contains("tags"))
    #expect(csv.contains("sci-fi"))
}
