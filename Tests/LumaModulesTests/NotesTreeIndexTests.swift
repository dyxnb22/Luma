import Foundation
import Testing
@testable import LumaModules

@Test func notesTreeEmptyRootReturnsNilSnapshot() async {
    let index = NotesTreeIndex()
    await index.setRoot(nil)
    await index.warmup()
    let snapshot = await index.snapshot()
    #expect(snapshot == nil)
}

@Test func notesTreeWarmupPerformanceFor100Files() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    for index in 0..<100 {
        let url = root.appendingPathComponent("note-\(index).md")
        try "body".write(to: url, atomically: true, encoding: .utf8)
    }

    let treeIndex = NotesTreeIndex()
    await treeIndex.setRoot(root)

    let start = ContinuousClock.now
    await treeIndex.warmup()
    let elapsed = start.duration(to: .now)

    #expect(elapsed < .milliseconds(200))

    try? FileManager.default.removeItem(at: root)
}

@Test func notesTreeFoldersPrecedeNotes() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "z".write(to: root.appendingPathComponent("zebra.md"), atomically: true, encoding: .utf8)
    try FileManager.default.createDirectory(at: root.appendingPathComponent("Alpha"), withIntermediateDirectories: true)

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let snapshot = await index.snapshot()
    #expect(snapshot?.children.count == 2)
    #expect(snapshot?.children[0].kind == .folder)
    #expect(snapshot?.children[0].name == "Alpha")
    #expect(snapshot?.children[1].kind == .note)
    #expect(snapshot?.children[1].name == "zebra")

    try? FileManager.default.removeItem(at: root)
}

@Test func notesTreePrefixSearch() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "a".write(to: root.appendingPathComponent("Tree.md"), atomically: true, encoding: .utf8)
    try "b".write(to: root.appendingPathComponent("Trunk.md"), atomically: true, encoding: .utf8)
    try "c".write(to: root.appendingPathComponent("Other.md"), atomically: true, encoding: .utf8)

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let matches = await index.search(prefix: "tr")
    #expect(matches.map(\.name).sorted() == ["Tree", "Trunk"])

    try? FileManager.default.removeItem(at: root)
}

@Test func notesTreeFuzzySearchRanksTighterMatchesFirst() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "a".write(to: root.appendingPathComponent("tree.md"), atomically: true, encoding: .utf8)
    try "b".write(to: root.appendingPathComponent("t-r-e-e-long.md"), atomically: true, encoding: .utf8)

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let matches = await index.search(fuzzy: "tree")
    #expect(matches.first?.name == "tree")

    try? FileManager.default.removeItem(at: root)
}
