import Foundation
import Testing
@testable import LumaModules

@Test func snippetsStoreRoundTrips() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("snippets-test-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = SnippetsStore(persistenceURL: url)
    let created = try await store.add(title: "Docker prune", content: "docker system prune -af", tags: ["docker", "cleanup"])
    #expect(created.title == "Docker prune")
    #expect(created.tags == ["docker", "cleanup"])

    let reloaded = SnippetsStore(persistenceURL: url)
    let items = await reloaded.all()
    #expect(items.count == 1)
    #expect(items[0].id == created.id)
    #expect(items[0].content == "docker system prune -af")
}

@Test func snippetsStoreRecordUsageIncrementsCounter() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("snippets-usage-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = SnippetsStore(persistenceURL: url)
    let snippet = try await store.add(title: "Test", content: "echo hi", tags: [])
    let updated = try await store.recordUsage(id: snippet.id)
    #expect(updated.usageCount == 1)
    try await store.flush()

    let again = SnippetsStore(persistenceURL: url)
    let items = await again.all()
    #expect(items[0].usageCount == 1)
}

@Test func snippetsStoreDuplicateCreatesCopy() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("snippets-dup-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }

    let store = SnippetsStore(persistenceURL: url)
    let original = try await store.add(title: "Original", content: "content", tags: ["a"])
    let copy = try await store.duplicate(id: original.id)
    #expect(copy.id != original.id)
    #expect(copy.title == "Original Copy")
    #expect(copy.content == original.content)
    #expect(await store.all().count == 2)
}
