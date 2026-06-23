import Foundation
import LumaCore
import Testing
@testable import LumaModules
@testable import LumaServices

@Test func frontmatterParserReadsCommonFields() {
    let markdown = """
    ---
    title: Hello
    type: reading
    tags: [swift, notes]
    pinned: true
    ---

    Body
    """
    let fields = FrontmatterParser.parse(markdown)
    #expect(fields.title == "Hello")
    #expect(fields.type == "reading")
    #expect(fields.tags == ["swift", "notes"])
    #expect(fields.pinned == true)
}

@Test func frontmatterParserIgnoresInvalidYaml() {
    let markdown = "No frontmatter here"
    let fields = FrontmatterParser.parse(markdown)
    #expect(fields == .empty)
}

@Test func notesMetaIndexFiltersByTagAndType() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let reading = """
    ---
    type: reading
    tags: swift
    pinned: true
    ---
    """
    try reading.write(to: root.appendingPathComponent("Book.md"), atomically: true, encoding: .utf8)
    try "plain".write(to: root.appendingPathComponent("Plain.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let treeIndex = NotesTreeIndex()
    await treeIndex.setRoot(root)
    await treeIndex.warmup()
    let metaIndex = NotesMetaIndex()
    await metaIndex.rebuild(from: await treeIndex.snapshot())

    let tagged = await metaIndex.notes(withTag: "swift")
    #expect(tagged.count == 1)
    #expect(tagged.first?.name == "Book")

    let typed = await metaIndex.notes(withType: "reading")
    #expect(typed.count == 1)

    let pinned = await metaIndex.pinnedNotes()
    #expect(pinned.count == 1)
}

@Test func notesQueryParserMetaQualifiers() {
    #expect(NotesQueryParser.parse(payload: "tag:swift", knownTemplates: []) == .metaSearch(NotesMetaFilter(tag: "swift")))
    #expect(NotesQueryParser.parse(payload: "type:reading", knownTemplates: []) == .metaSearch(NotesMetaFilter(type: "reading")))
    #expect(NotesQueryParser.parse(payload: "review week", knownTemplates: []) == .reviewWeek)
    #expect(NotesQueryParser.parse(payload: "doctor", knownTemplates: []) == .doctor)
}

@Test func notesModuleMetaSearchUsesMemoryIndex() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try """
    ---
    type: reading
    ---
    """.write(to: root.appendingPathComponent("Book.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()
    let metaIndex = NotesMetaIndex()
    await metaIndex.rebuild(from: await index.snapshot())

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    let config = NotesRootConfigStore(fileURL: configURL)
    try await config.save(NotesRootConfig(root: root, expandedFolders: []))

    let module = NotesModule(index: index, config: config, metaIndex: metaIndex)
    await module.reloadFromConfig()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))

    let result = await module.handle(Query(raw: "n type:reading", sequence: 1), context: context)
    #expect(result.items.first?.title == "Book")
}
