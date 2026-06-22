import Foundation
import Testing
@testable import LumaCore

private struct TestItem: Codable, Identifiable, Sendable, Hashable {
    let id: UUID
    var name: String
}

@Test func jsonFileStoreRoundTripsAndQuarantinesCorruptData() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("json-store-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = JSONFileStore<TestItem>(url: url)
    let item = TestItem(id: UUID(), name: "alpha")
    try await store.mutate { items in
        items.append(item)
    }

    let reloaded = JSONFileStore<TestItem>(url: url)
    let items = await reloaded.items
    #expect(items.count == 1)
    #expect(items[0].id == item.id)

    try "not json".write(to: url, atomically: true, encoding: .utf8)
    let afterCorrupt = JSONFileStore<TestItem>(url: url)
    #expect(await afterCorrupt.items.isEmpty)
}

@Test func jsonFileStoreBuffersWrites() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("json-buffer-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = JSONFileStore<TestItem>(url: url)
    let item = TestItem(id: UUID(), name: "buffered")
    try await store.mutateBuffered({ items in
        items.append(item)
    }, flushEvery: 10, maxInterval: 30)

    #expect(FileManager.default.fileExists(atPath: url.path) == false)

    try await store.flushIfNeeded()
    let reloaded = JSONFileStore<TestItem>(url: url)
    #expect(await reloaded.items.count == 1)
}
