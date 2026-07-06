import Foundation
import Testing
import LumaCore
import LumaModules
import LumaServices

private final class CountingMenuBarTreeClient: MenuBarTreeClient, @unchecked Sendable {
    let count: Int
    private(set) var callCount = 0

    init(count: Int) { self.count = count }

    func staleMenuItemCountForFrontmost() async -> Int {
        callCount += 1
        return count
    }
}

private struct AuthorizedRemindersClient: RemindersClient {
    func authorization() async -> RemindersAuthorization { .authorized }
    func requestAccess() async -> RemindersAuthorization { .authorized }
    func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func noDueDate(limit: Int) async throws -> [ReminderSnapshot] { [] }
    func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot {
        throw RemindersServiceError.accessDenied
    }
    func complete(id: String) async throws {}
    func uncomplete(id: String) async throws {}
    func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot {
        throw RemindersServiceError.accessDenied
    }
    func storeChanges() async -> AsyncStream<Void> { AsyncStream { $0.finish() } }
}

@Test func commandsDoctorUsesInjectedPlatformClients() async throws {
    let menuBarTree = CountingMenuBarTreeClient(count: 7)
    let module = CommandsModule()
    await module.warmup(ModuleContext(
        logger: DoctorTestLogger(),
        metrics: NoopMetricsClient(),
        database: DoctorTestDatabase(),
        pasteboard: NoopPasteboardClient(),
        accessibility: DoctorAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: DoctorTestConfig(),
        reminders: AuthorizedRemindersClient(),
        menuBarTree: menuBarTree
    ))

    let parsed = ParsedCommand(trigger: "cmd", payload: "doctor", module: .commands)
    let handleResult = await module.handle(
        Query(raw: "cmd doctor", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(40)))
    )

    #expect(handleResult.items.contains { $0.id.key == "doctor" })

    let payload = (try? ModuleActionCoding.encode(CommandsAction.doctor)) ?? Data()
    let action = Action(
        id: ActionID(module: .commands, key: "doctor"),
        title: "Global Doctor",
        kind: .custom(payload: payload, handler: .commands)
    )
    try await module.perform(action, context: ActionContext(
        logger: DoctorTestLogger(),
        metrics: NoopMetricsClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: DoctorAccessibilityClient()
    ))
    #expect(menuBarTree.callCount == 1)
}

private struct DoctorTestLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private struct DoctorTestDatabase: DatabaseClient {}

private struct DoctorAccessibilityClient: AccessibilityClient {
  func isTrusted() async -> Bool { true }
  func requestPermission() async {}
  func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
  func insert(text: String) async {}
  func replaceSelectedText(with text: String) async -> Bool { false }
  func applyWindowLayout(_ preset: String) async {}
}

private struct DoctorTestConfig: ConfigurationClient {
  func enabledModules() async -> Set<ModuleIdentifier>? { nil }
  func clipboardMaxEntries() async -> Int { 500 }
  func clipboardMaxAgeDays() async -> Int { 7 }
  func clipboardMaxEntrySizeKB() async -> Int { 100 }
  func clipboardHistoryEnabled() async -> Bool { true }
  func clipboardIgnoredBundleIDs() async -> [String] { [] }
  func clipboardPasteBehavior() async -> String { "pasteDirectly" }
  func translationTargetLanguage() async -> String { "en" }
  func secretsAutoClearSeconds() async -> Int { 10 }
  func secretsRelockTimeoutSeconds() async -> Int { 300 }
  func secretsRequireUnlockOnLaunch() async -> Bool { true }
}
