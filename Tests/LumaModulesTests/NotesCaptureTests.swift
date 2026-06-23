import Foundation
import LumaCore
import Testing
@testable import LumaModules
@testable import LumaServices

@Test func notesQueryParserRecognizesTriggers() {
    #expect(NotesQueryParser.extractPayload(raw: "n new idea") == "new idea")
    #expect(NotesQueryParser.extractPayload(raw: "note meeting") == "meeting")
    #expect(NotesQueryParser.extractPayload(raw: "notes daily") == "daily")
    #expect(NotesQueryParser.extractPayload(raw: "todo buy") == nil)
}

@Test func notesQueryParserNewWithoutTemplate() {
    let parsed = NotesQueryParser.parse(payload: "new My Idea", knownTemplates: ["reading"])
    #expect(parsed == .new(title: "My Idea", template: nil))
}

@Test func notesQueryParserNewWithTemplate() {
    let parsed = NotesQueryParser.parse(payload: "new reading Deep Work", knownTemplates: ["reading"])
    #expect(parsed == .new(title: "Deep Work", template: "reading"))
}

@Test func notesQueryParserDailyAliases() {
    #expect(NotesQueryParser.parse(payload: "daily", knownTemplates: []) == .daily)
    #expect(NotesQueryParser.parse(payload: "today", knownTemplates: []) == .daily)
}

@Test func templateRendererSubstitutesVariables() {
    var components = DateComponents()
    components.year = 2026
    components.month = 6
    components.day = 23
    let calendar = Calendar(identifier: .gregorian)
    let date = calendar.date(from: components)!

    let rendered = TemplateRenderer.render(
        "# {{title}}\nDate: {{date}}\nWeek: {{week}}",
        title: "Hello",
        now: date,
        calendar: calendar
    )
    #expect(rendered.contains("# Hello"))
    #expect(rendered.contains("Date: 2026-06-23"))
    #expect(rendered.contains("Week: 2026-W26"))
}

@Test func notesModuleAcceptsNTrigger() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "body".write(to: root.appendingPathComponent("Meeting.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    let config = NotesRootConfigStore(fileURL: configURL)
    try await config.save(NotesRootConfig(root: root, expandedFolders: []))

    let module = NotesModule(index: index, config: config)
    await module.reloadFromConfig()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))

    let withN = await module.handle(Query(raw: "n meeting", sequence: 1), context: context)
    #expect(withN.items.first?.title == "Meeting")
}

@Test func notesModuleNewCaptureRow() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    let config = NotesRootConfigStore(fileURL: configURL)
    try await config.save(NotesRootConfig(root: root, expandedFolders: []))

    let module = NotesModule(index: index, config: config)
    await module.reloadFromConfig()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))

    let result = await module.handle(Query(raw: "n new Quick thought", sequence: 1), context: context)
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Quick thought")
    #expect(result.items.first?.subtitle?.contains("Inbox") == true)
}

@Test func notesModuleHandleUsesCachedConfigAfterReload() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "body".write(to: root.appendingPathComponent("Meeting.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: configURL) }
    let config = NotesRootConfigStore(fileURL: configURL)
    try await config.save(NotesRootConfig(root: root, expandedFolders: []))

    let module = NotesModule(index: index, config: config)
    await module.reloadFromConfig()
    try await config.save(.empty)

    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    let result = await module.handle(Query(raw: "n meeting", sequence: 1), context: context)
    #expect(result.items.first?.title == "Meeting")
}

@Test func notesActionsCreatesInboxNote() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()
    let actions = NotesActions(index: index)

    let url = try await actions.createNoteInInbox(title: "Capture", root: root, inboxFolderName: "Inbox")
    #expect(url.lastPathComponent == "Capture.md")
    #expect(FileManager.default.fileExists(atPath: url.path))
}

@Test func notesActionsOpenOrCreateDailyNote() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()
    let actions = NotesActions(index: index)

    let url = try await actions.openOrCreateDailyNote(root: root, dailyFolderName: "Daily")
    #expect(url.deletingLastPathComponent().lastPathComponent == "Daily")
    #expect(url.pathExtension == "md")
    let body = try String(contentsOf: url, encoding: .utf8)
    #expect(body.hasPrefix("# "))
}

@Test func notesActionsCreateFromTemplate() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let templates = root.appendingPathComponent("_templates", isDirectory: true)
    try FileManager.default.createDirectory(at: templates, withIntermediateDirectories: true)
    try "# {{title}}\nType: reading".write(to: templates.appendingPathComponent("reading.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()
    let actions = NotesActions(index: index)
    let template = NotesTemplateInfo(name: "reading", url: templates.appendingPathComponent("reading.md"))

    let url = try await actions.createNoteFromTemplate(
        template: template,
        title: "Deep Work",
        root: root,
        inboxFolderName: "Inbox"
    )
    let body = try String(contentsOf: url, encoding: .utf8)
    #expect(body.contains("# Deep Work"))
    #expect(body.contains("Type: reading"))
}

@Test func notesActionsFindBacklinksScansBodies() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "see [[Target]] here".write(to: root.appendingPathComponent("Source.md"), atomically: true, encoding: .utf8)
    try "plain".write(to: root.appendingPathComponent("Target.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()
    let actions = NotesActions(index: index)

    let backlinks = await actions.findBacklinks(to: "Target")
    #expect(backlinks.count == 1)
    #expect(backlinks.first?.lastPathComponent == "Source.md")
}

@Test func notesModuleHandleDoesNotMatchBareFilename() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try "body".write(to: root.appendingPathComponent("Meeting.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    let config = NotesRootConfigStore(fileURL: configURL)
    try await config.save(NotesRootConfig(root: root, expandedFolders: []))

    let module = NotesModule(index: index, config: config)
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))

    let noPrefix = await module.handle(Query(raw: "meeting", sequence: 1), context: context)
    #expect(noPrefix.items.isEmpty)
}

@Test func notesTemplateStoreScansTemplates() throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let templates = root.appendingPathComponent("_templates", isDirectory: true)
    try FileManager.default.createDirectory(at: templates, withIntermediateDirectories: true)
    try "# {{title}}".write(to: templates.appendingPathComponent("reading.md"), atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: root) }

    let found = NotesTemplateStore.scanTemplates(root: root, folderName: "_templates")
    #expect(found.map(\.name) == ["reading"])
}
