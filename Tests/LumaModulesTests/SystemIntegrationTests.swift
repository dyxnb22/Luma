import Foundation
import Testing
@testable import LumaModules

@Test func keychainSecretsStoreRoundTripsValue() throws {
    let service = "app.luma.tests.\(UUID().uuidString)"
    let account = "integration"
    let store = KeychainSecretsStore(service: service)
    defer { try? store.delete(account: account) }

    try store.save(value: "secret-value", account: account)
    #expect(try store.read(account: account) == "secret-value")
    try store.delete(account: account)
}

@Test func notesTreeIndexCreatesAndSearchesMarkdown() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let url = root.appendingPathComponent("Luma Note.md")
    try "Links to [[Other Note]]".write(to: url, atomically: true, encoding: .utf8)

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let snapshot = await index.snapshot()
    #expect(snapshot?.children.count == 1)
    #expect(snapshot?.children.first?.name == "Luma Note")

    let matches = await index.search(prefix: "luma")
    #expect(matches.first?.name == "Luma Note")

    try? FileManager.default.removeItem(at: root)
}

@Test func wordbookStoreReadsExistingWordbotDatabaseWhenPresent() async throws {
    let url = URL(fileURLWithPath: "/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3")
    guard FileManager.default.fileExists(atPath: url.path) else { return }
    let store = WordbookStore(dbURL: url)
    let count = try await store.count()
    #expect(count > 0)
    let matches = try await store.search("latency", limit: 5)
    #expect(matches.contains { $0.term == "latency" })
}
