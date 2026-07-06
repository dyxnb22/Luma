import Foundation
import Testing
import LumaCore
@testable import LumaModules
import LumaServices

@Test func killProcessFindsInjectedPreviewRecord() async {
    let preview = RunningProcessRecord(
        pid: 4242,
        bundleID: "com.apple.Preview",
        name: "Preview",
        launchDate: nil,
        residentBytes: 64 * 1_048_576
    )
    let service = RunningProcessService(fixedRecords: [preview])
    let module = KillProcessModule(service: service)
    await module.seedCacheForTesting([preview], fetchedAt: ContinuousClock.now)
    await module.warmup(ModuleContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        database: NoopDatabaseClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: NoopConfigurationClient()
    ))

    let query = Query(
        raw: "kill preview",
        sequence: 1,
        command: ParsedCommand(trigger: "kill", payload: "preview", module: .killProcess)
    )
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1)))
    #expect(result.items.contains(where: { $0.title == "Preview" }))
}

private struct NoopLoggingClient: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private struct NoopDatabaseClient: DatabaseClient {}

private struct NoopConfigurationClient: ConfigurationClient {
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
