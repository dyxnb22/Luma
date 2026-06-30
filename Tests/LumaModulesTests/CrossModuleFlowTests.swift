import Foundation
import Testing
import LumaCore
@testable import LumaModules
@testable import LumaServices

@Test func snippetDraftFromClipboardBuildsTriggerAndTitle() {
    let draft = SnippetDraft.fromClipboard("Hello World\nsecond line")
    #expect(draft.title == "Hello World")
    #expect(draft.trigger.hasPrefix(";"))
    #expect(draft.content.contains("\n"))
    #expect(draft.tags == ["clipboard"])
}

@Test func snippetDraftFromClipboardFlagsLongContent() {
    let long = String(repeating: "x", count: 2_500)
    let draft = SnippetDraft.fromClipboard(long)
    #expect(draft.isLongClipboardClip)
}

@Test func urlQuicklinkDraftFromURLUsesHostSlug() throws {
    let url = try #require(URL(string: "https://www.github.com/copilot"))
    let draft = URLQuicklinkDraft.from(url: url)
    #expect(draft.trigger == "github")
    #expect(draft.name == "github.com")
    #expect(draft.urlTemplate == url.absoluteString)
}

@Test func urlTextParserFindsHTTPURLInMixedText() throws {
    let text = "see https://example.com/docs for more"
    let url = try #require(URLTextParser.firstHTTPURL(in: text))
    #expect(url.host == "example.com")
}

@Test func textQuicklinkDraftSourceFindsEmbeddedHTTPURL() {
    let draft = TextQuicklinkDraftSource(
        text: "see https://developer.apple.com/documentation for more"
    ).quicklinkDraft()
    #expect(draft?.name == "developer.apple.com")
    #expect(draft?.trigger == "developer")
    #expect(draft?.urlTemplate == "https://developer.apple.com/documentation")
}

@Test func notesCaptureTextToDailyNoteAppendsContent() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
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

    let clipText = "Clipboard capture line"
    let outcome = await module.captureTextToDailyNote(clipText)
    guard case .appended(let url) = outcome else {
        Issue.record("Expected appended outcome, got \(outcome)")
        return
    }
    let body = try String(contentsOf: url, encoding: .utf8)
    #expect(body.contains(clipText))
}

@Test func notesCaptureWithoutRootReturnsRootNotConfigured() async {
    let index = NotesTreeIndex()
    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: configURL) }
    let config = NotesRootConfigStore(fileURL: configURL)
    try? await config.save(.empty)

    let module = NotesModule(index: index, config: config)
    let outcome = await module.captureTextToDailyNote("hello")
    guard case .rootNotConfigured = outcome else {
        Issue.record("Expected rootNotConfigured")
        return
    }
}

@Test func quicklinksStoreDetectsTriggerConflict() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("quicklinks-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }
    try "[]".write(to: url, atomically: true, encoding: .utf8)
    let store = QuicklinksStore(url: url, fileManager: .default)
    _ = try await store.add(Quicklink(name: "GitHub", trigger: "gh", urlTemplate: "https://github.com/search?q={{query}}"))

    let conflict = await store.conflictingQuicklink(trigger: "gh")
    #expect(conflict?.name == "GitHub")

    let editing = await store.all().first
    let noSelfConflict = await store.conflictingQuicklink(trigger: "gh", excluding: editing?.id)
    #expect(noSelfConflict == nil)
}

@Test func snippetsPrepareDraftActionEncodesRoundTrip() throws {
    let draft = SnippetDraft.fromClipboard("sample clip")
    let payload = try ModuleActionCoding.encode(SnippetsAction.prepareDraft(draft))
    let decoded = try ModuleActionCoding.decode(SnippetsAction.self, from: payload)
    guard case .prepareDraft(let roundTrip) = decoded else {
        Issue.record("Expected prepareDraft")
        return
    }
    #expect(roundTrip == draft)
}

@Test func quicklinksPrepareDraftActionEncodesRoundTrip() throws {
    let draft = URLQuicklinkDraft(name: "Example", trigger: "ex", urlTemplate: "https://example.com")
    let payload = try ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))
    let decoded = try ModuleActionCoding.decode(QuicklinksAction.self, from: payload)
    #expect(decoded == .prepareDraft(draft))
}

@Test func notesCaptureToDailyActionEncodesRoundTrip() throws {
    let payload = try ModuleActionCoding.encode(NotesAction.captureToDaily(text: "clip"))
    let decoded = try ModuleActionCoding.decode(NotesAction.self, from: payload)
    guard case .captureToDaily(let text) = decoded else {
        Issue.record("Expected captureToDaily")
        return
    }
    #expect(text == "clip")
}

@Test func quicklinksStoreDetectsDuplicateURL() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("quicklinks-dup-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url) }
    try "[]".write(to: url, atomically: true, encoding: .utf8)
    let store = QuicklinksStore(url: url, fileManager: .default)
    _ = try await store.add(Quicklink(name: "Example", trigger: "ex", urlTemplate: "https://example.com/docs"))

    let duplicate = await store.duplicateQuicklink(urlTemplate: "https://example.com/docs")
    #expect(duplicate?.name == "Example")

    let noSelfConflict = await store.duplicateQuicklink(
        urlTemplate: "https://example.com/docs",
        excluding: duplicate?.id
    )
    #expect(noSelfConflict == nil)
}

@Test func snippetsStoreDetectsTriggerAndSimilarContent() async throws {
    let snippetsURL = FileManager.default.temporaryDirectory.appendingPathComponent("snippets-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: snippetsURL) }
    let store = SnippetsStore(persistenceURL: snippetsURL)
    let saved = try await store.add(title: "Addr", content: "123 Main Street", tags: [], trigger: ";addr")

    let triggerConflict = await store.conflictingSnippet(trigger: ";addr")
    #expect(triggerConflict?.id == saved.id)

    let similar = await store.similarSnippet(content: "123 Main Street")
    #expect(similar?.id == saved.id)
}

@Test func projectNotesPathsFindsExistingNote() throws {
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: dir) }
    let notes = dir.appendingPathComponent("NOTES.md")
    try "hello".write(to: notes, atomically: true, encoding: .utf8)
    let found = ProjectNotesPaths.existingNotePath(projectPath: dir.path, projectName: "Demo")
    #expect(found == notes.path)
}

@Test func quicklinksURLValidationRequiresProtocolForBareHosts() {
    #expect(QuicklinksStore.validateURLTemplate("example.com")?.contains("http") == true)
    #expect(QuicklinksStore.validateURLTemplate("https://example.com/{{query}}") == nil)
}

@Test func clipboardTextOpsClassifiesJSONAndTransforms() {
    let json = "{\"a\":1}"
    #expect(ClipboardTextOps.classify(json) == .json)
    #expect(ClipboardTextOps.detectJSON(json)?.contains("\"a\"") == true)
    let text = "  hello\n\nworld  "
    #expect(ClipboardTextOps.collapseLines(text) == "hello world")
}
