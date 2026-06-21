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

@Test func notesVaultStoreCreatesScansAndIndexesMarkdown() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let store = NotesVaultStore(vaultURL: root)
    let url = try await store.create(title: "Luma Note", body: "Links to [[Other Note]] #luma")
    #expect(url.pathExtension == "md")

    let notes = await store.scan()
    #expect(notes.count == 1)
    #expect(notes.first?.title == "Luma Note")

    let graph = await store.graph()
    #expect(graph.edges.contains { edge in
        edge.from.hasSuffix("/Luma Note.md") && edge.to == "Other Note" && edge.kind == "wiki"
    })
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
