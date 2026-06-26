import Foundation
import Testing
import LumaCore
import LumaModules

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

@Test func wordbookModuleReviewCommandReturnsStarterRow() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    _ = try await store.upsertWords([WordbookTestFixtures.newWord(term: "alpha")])
    let module = WordbookModule(store: store)

    let parsed = ParsedCommand(trigger: "word", payload: "review", module: .wordbook)
    let result = await module.handle(
        Query(raw: "word review", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )
    #expect(!result.items.isEmpty)
    #expect(result.items[0].id.key == "review")
    #expect(result.items[0].primaryAction.title == "Start Review")
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
