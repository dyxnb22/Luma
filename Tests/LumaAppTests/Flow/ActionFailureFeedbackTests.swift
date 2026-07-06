import Foundation
import LumaCore
import Testing

private struct ActionFailureTestLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private struct NoopDatabaseClient: DatabaseClient {}
private struct NoopConfigurationClient: ConfigurationClient {
    func enabledModules() async -> Set<ModuleIdentifier>? { nil }
    func clipboardMaxEntries() async -> Int { 100 }
    func clipboardMaxAgeDays() async -> Int { 30 }
    func clipboardMaxEntrySizeKB() async -> Int { 512 }
    func clipboardHistoryEnabled() async -> Bool { true }
    func clipboardIgnoredBundleIDs() async -> [String] { [] }
    func clipboardPasteBehavior() async -> String { "pasteDirectly" }
    func translationTargetLanguage() async -> String { "en" }
    func secretsAutoClearSeconds() async -> Int { 60 }
    func secretsRelockTimeoutSeconds() async -> Int { 300 }
    func secretsRequireUnlockOnLaunch() async -> Bool { false }
}

@Test func actionFailureSurfacesAccessibilityMessage() async {
    let host = ModuleHost(context: ModuleContext(
        logger: ActionFailureTestLogger(),
        metrics: NoopMetricsClient(),
        database: NoopDatabaseClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: NoopConfigurationClient()
    ))
    let executor = ActionExecutor(
        host: host,
        context: ActionContext(
            logger: ActionFailureTestLogger(),
            metrics: NoopMetricsClient(),
            pasteboard: NoopPasteboardClient(),
            accessibility: NoopAccessibilityClient()
        ),
        usage: InMemoryUsageTracker()
    )
    let result = await executor.run(
        Action(id: ActionID(module: .snippets, key: "paste"), title: "Paste", kind: .insertText("hello")),
        for: ResultID(module: .snippets, key: "paste")
    )
    #expect(result == .failure(message: "Accessibility permission is required.", recoverable: true))
}
