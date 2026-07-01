import Foundation
import Testing
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@Test func snippetsModuleExtractPayload() {
    #expect(SnippetsModule.extractPayload(raw: "s") == "")
    #expect(SnippetsModule.extractPayload(raw: "s git") == "git")
    #expect(SnippetsModule.extractPayload(raw: "chrome") == nil)
}

@Test func clipboardModuleExtractPayload() {
    #expect(ClipboardModule.extractPayload(raw: "clip") == "")
    #expect(ClipboardModule.extractPayload(raw: "cb jwt") == "jwt")
}

@Test func secretsModuleExtractPayload() {
    #expect(SecretsModule.extractPayload(raw: "sec") == "")
    #expect(SecretsModule.extractPayload(raw: "sec unlock") == "unlock")
    #expect(SecretsModule.extractPayload(raw: "secret aws") == "aws")
}

@Test func wordbookModuleExtractPayload() {
    #expect(WordbookModule.extractPayload(raw: "wb") == "")
    #expect(WordbookModule.extractPayload(raw: "word review") == "review")
}

@Test func wordbookModuleReviewCommandReturnsStartRow() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    _ = try await store.upsertWords([WordbookTestFixtures.newWord(term: "alpha")])
    let module = WordbookModule(store: store)

    let parsed = ParsedCommand(trigger: "word", payload: "review", module: .wordbook)
    let result = await module.handle(
        Query(raw: "word review", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Start Review")
    if case .openModuleDetail(let module, _) = result.items.first?.primaryAction.kind {
        #expect(module.rawValue == "luma.wordbook")
    } else {
        Issue.record("Expected openModuleDetail")
    }
}

@Test func wordbookBareCommandReturnsNoStarterRow() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let module = WordbookModule(store: store)
    let parsed = ParsedCommand(trigger: "word", payload: "", module: .wordbook)
    let result = await module.handle(
        Query(raw: "word", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )
    #expect(!result.items.contains { $0.id.key == "review" })
}

@Test func snippetsNewBareCommandReturnsCreateRow() async {
    let module = SnippetsModule()
    await module.warmup(ModuleContext(
        logger: LumaLogger(category: "test"),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(config: ConfigurationStore()),
        config: ConfigurationStore()
    ))
    let parsed = ParsedCommand(trigger: "s", payload: "new", module: .snippets)
    let result = await module.handle(
        Query(raw: "s new", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(20)))
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.subtitle == "Untitled")
}

@Test func snippetsNewCommandReturnsCreateRow() async {
    let module = SnippetsModule()
    await module.warmup(ModuleContext(
        logger: LumaLogger(category: "test"),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(config: ConfigurationStore()),
        config: ConfigurationStore()
    ))
    let parsed = ParsedCommand(trigger: "s", payload: "new gitignore", module: .snippets)
    let result = await module.handle(
        Query(raw: "s new gitignore", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(20)))
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.subtitle == "gitignore")
    if case .openModuleDetail(let module, _) = result.items.first?.primaryAction.kind {
        #expect(module.rawValue == "luma.snippets")
    } else {
        Issue.record("Expected openModuleDetail")
    }
}

@Test func snippetsPasteWritesExpandedContentToPasteboard() async throws {
    let pasteboard = SnippetsTestPasteboard()
    let module = SnippetsModule()
    let context = ModuleContext(
        logger: LumaLogger(category: "test"),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: pasteboard,
        accessibility: SnippetsTestAccessibility(),
        fileSystem: FSEventsService(),
        translation: TranslationService(config: ConfigurationStore()),
        config: ConfigurationStore()
    )
    await module.warmup(context)
    let snippet = try await module.add(title: "Date", content: "Hello {{date}}", tags: [])
    let payload = try ModuleActionCoding.encode(SnippetsAction.paste(id: snippet.id))
    let action = Action(
        id: ActionID(module: .snippets, key: "paste"),
        title: "Paste",
        kind: .custom(payload: payload, handler: .snippets)
    )
    try await module.perform(
        action,
        context: ActionContext(
            logger: LumaLogger(category: "test"),
            metrics: LumaMetrics(),
            pasteboard: pasteboard,
            accessibility: SnippetsTestAccessibility()
        )
    )
    let recorded = await pasteboard.snapshot()
    #expect(recorded?.contains("Hello ") == true)
    #expect(recorded?.contains("{{date}}") == false)
}

private actor SnippetsTestPasteboard: PasteboardClient {
    private var text: String?

    func write(_ string: String) async { text = string }
    func writeSecure(_ string: String, clearAfterSeconds: Int) async { await write(string) }
    func writeImage(data: Data, pasteboardType: String) async {}
    func writeFileURLs(_ urls: [URL]) async {}

    func readString() async -> String? { text }

    func snapshot() -> String? { text }
}

private struct SnippetsTestAccessibility: AccessibilityClient {
    func isTrusted() async -> Bool { true }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func applyWindowLayout(_ preset: String) async {}
}

@Test func moduleHelpReadsFromRegistry() {
    let lines = ModuleHelp.lines(for: .media)
    #expect(lines.first?.contains("rec") == true)
    #expect(!lines.joined().contains("m / media"))
}

@Test func appsModuleExtractPayload() {
    #expect(AppsModule.extractPayload(raw: "app") == "")
    #expect(AppsModule.extractPayload(raw: "app top") == "top")
    #expect(AppsModule.extractPayload(raw: "chrome") == nil)
}

@Test func commandsModuleExtractPayload() {
    #expect(CommandsModule.extractPayload(raw: "settings") == "")
    #expect(CommandsModule.extractPayload(raw: "prefs") == "")
    #expect(CommandsModule.extractPayload(raw: "open-settings") == "")
    #expect(CommandsModule.extractPayload(raw: "quit") == "")
}

@Test func commandsModuleBuiltInActionsUseHostClient() async throws {
    let host = RecordingHostClient()
    let module = CommandsModule()
    let action = Action(
        id: ActionID(module: .commands, key: "open-settings"),
        title: "Open Settings",
        kind: .custom(payload: Data("open-settings".utf8), handler: .commands)
    )
    try await module.perform(
        action,
        context: ActionContext(
            logger: LumaLogger(category: "test"),
            metrics: LumaMetrics(),
            pasteboard: SnippetsTestPasteboard(),
            accessibility: SnippetsTestAccessibility(),
            host: host
        )
    )
    #expect(await host.openedSettings == true)
}

private actor RecordingHostClient: HostClient {
    private(set) var openedSettings = false
    private(set) var reloadedModules = false
    private(set) var quitRequested = false

    func openSettings() async { openedSettings = true }
    func reloadModules() async { reloadedModules = true }
    func quitHost() async { quitRequested = true }
}

@Test func todoBareCommandIncludesOpenDetailRow() async {
    let module = TodoModule()
    let context = ModuleContext(
        logger: LumaLogger(category: "test"),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(config: ConfigurationStore()),
        config: ConfigurationStore(),
        reminders: AuthorizedEmptyRemindersClient()
    )
    await module.warmup(context)
    let parsed = ParsedCommand(trigger: "todo", payload: "", module: .todo)
    let result = await module.handle(
        Query(raw: "todo", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(60)))
    )
    #expect(result.items.contains { $0.id.key == "open-detail" })
    if case .openModuleDetail(let moduleID, _) = result.items.first(where: { $0.id.key == "open-detail" })?.primaryAction.kind {
        #expect(moduleID.rawValue == "luma.todo")
    } else {
        Issue.record("Expected openModuleDetail for Open Todo row")
    }
}

@Test func mediaBareCommandOpensRecordsWhenLibraryEmpty() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("media-bare-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }
    let module = MediaModule(store: MediaStore(persistenceURL: url))
    await module.warmup(ModuleContext(
        logger: LumaLogger(category: "test"),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(config: ConfigurationStore()),
        config: ConfigurationStore()
    ))
    let parsed = ParsedCommand(trigger: "m", payload: "", module: .media)
    let result = await module.handle(
        Query(raw: "m", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(30)))
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Records")
}

private struct AuthorizedEmptyRemindersClient: RemindersClient {
    func authorization() async -> RemindersAuthorization { .authorized }
    func requestAccess() async -> RemindersAuthorization { .authorized }
    func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func noDueDate(limit: Int) async throws -> [ReminderSnapshot] { [] }
    func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot {
        ReminderSnapshot(id: "1", title: title, dueDate: dueDate, isCompleted: false, calendarTitle: "Inbox")
    }
    func complete(id: String) async throws {}
    func uncomplete(id: String) async throws {}
    func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot {
        ReminderSnapshot(id: id, title: title, dueDate: dueDate, isCompleted: false, calendarTitle: "Inbox")
    }
    func storeChanges() async -> AsyncStream<Void> {
        AsyncStream { $0.finish() }
    }
}

@Test func commandsModuleMatchesBuiltInKeys() async {
    let module = CommandsModule()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(20)))
    let commands = ModuleIdentifier(rawValue: "luma.commands")
    let parsed = ParsedCommand(trigger: "open-settings", payload: "", module: commands)
    let result = await module.handle(
        Query(raw: "open-settings", sequence: 1, command: parsed),
        context: context
    )
    #expect(result.items.count == 1)
    #expect(result.items[0].id.key == "open-settings")
}
