import Foundation
import LumaCore
import Testing
@testable import LumaModules

@Test func notesDoctorFindsBrokenLinksAndDuplicates() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "links to [[Missing]]".write(to: root.appendingPathComponent("Source.md"), atomically: true, encoding: .utf8)
    try "other".write(to: root.appendingPathComponent("Dup.md"), atomically: true, encoding: .utf8)
    try FileManager.default.createDirectory(at: root.appendingPathComponent("Sub"), withIntermediateDirectories: true)
    try "x".write(to: root.appendingPathComponent("Sub/Dup.md"), atomically: true, encoding: .utf8)
    try """
    ---
    title: Bad
    """.write(to: root.appendingPathComponent("BadFrontmatter.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()
    let tree = await index.snapshot()

    let (issues, stats) = await NotesDoctor.diagnose(tree: tree, lastWarmupMilliseconds: 12)
    #expect(stats.noteCount == 4)
    #expect(issues.contains { $0.kind == .brokenLink && $0.message.contains("Missing") })
    #expect(issues.contains { $0.kind == .duplicateName })
    #expect(issues.contains { $0.kind == .frontmatter })
}

@Test func notesModuleDoctorCommand() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "see [[Ghost]]".write(to: root.appendingPathComponent("A.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    let config = NotesRootConfigStore(fileURL: configURL)
    try await config.save(NotesRootConfig(root: root, expandedFolders: []))

    let module = NotesModule(index: index, config: config)
    await module.reloadFromConfig()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(500)))

    let result = await module.handle(Query(raw: "n doctor", sequence: 1), context: context)
    #expect(result.items.first?.title.contains("notes") == true)
    #expect(result.items.contains { $0.subtitle?.contains("Ghost") == true })
}

@Test func notesPortabilityRecoversFromEmptyConfig() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: root.appendingPathComponent("Inbox"), withIntermediateDirectories: true)
    try "# Inbox note".write(to: root.appendingPathComponent("Inbox/Captured.md"), atomically: true, encoding: .utf8)
    try "---\ntype: reading\ntags: swift\n---\n".write(to: root.appendingPathComponent("Book.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-portability-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: configURL) }
    let configStore = NotesRootConfigStore(fileURL: configURL)

    let index = NotesTreeIndex()
    let metaIndex = NotesMetaIndex()
    let module = NotesModule(index: index, config: configStore, metaIndex: metaIndex)

    try await configStore.save(NotesRootConfig(root: root, expandedFolders: [root.path]))
    await module.reloadFromConfig()

    let beforeSearch = await module.handle(
        Query(raw: "n Book", sequence: 1),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )
    #expect(beforeSearch.items.first?.title == "Book")

    try await configStore.save(.empty)
    await module.reloadFromConfig()

    try await configStore.save(NotesRootConfig(root: root, expandedFolders: []))
    await module.reloadFromConfig()

    let warmupMs = await module.lastWarmupMilliseconds()
    #expect(warmupMs >= 0)

    let snapshot = await module.snapshot()
    #expect(snapshot != nil)

    let search = await module.handle(
        Query(raw: "n Book", sequence: 2),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )
    #expect(search.items.first?.title == "Book")

    let typed = await module.handle(
        Query(raw: "n type:reading", sequence: 3),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )
    #expect(typed.items.first?.title == "Book")

    let inbox = await module.inboxCount()
    #expect(inbox == 1)
}

@Test func notesQueryParserDoctor() {
    #expect(NotesQueryParser.parse(payload: "doctor", knownTemplates: []) == .doctor)
}
