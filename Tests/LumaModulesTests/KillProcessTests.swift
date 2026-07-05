import Darwin
import Foundation
import Testing
@testable import LumaModules
import LumaCore
import LumaServices

@Test func killProcessIndexFiltersSelfPID() {
    let selfPID = pid_t(42)
    let records = [
        RunningProcessRecord(pid: 42, bundleID: "luma", name: "Luma", launchDate: nil, residentBytes: nil),
        RunningProcessRecord(pid: 43, bundleID: "preview", name: "Preview", launchDate: nil, residentBytes: nil)
    ]
    #expect(KillProcessIndex.filtered(records, selfPID: selfPID).map(\.pid) == [43])
}

@Test func killProcessIndexSearchesLocalizedNameAndBundleID() {
    let records = [
        RunningProcessRecord(pid: 10, bundleID: "com.apple.Preview", name: "预览", launchDate: nil, residentBytes: nil)
    ]
    #expect(KillProcessIndex.search(records, query: "preview").first?.record.pid == 10)
    #expect(KillProcessIndex.search(records, query: "预览").first?.record.pid == 10)
}

@Test func killProcessIndexSearchesNameAndBundleID() {
    let records = [
        RunningProcessRecord(pid: 10, bundleID: "com.apple.Preview", name: "Preview", launchDate: nil, residentBytes: nil),
        RunningProcessRecord(pid: 11, bundleID: "com.apple.Safari", name: "Safari", launchDate: nil, residentBytes: nil)
    ]
    #expect(KillProcessIndex.search(records, query: "preview").first?.record.pid == 10)
    #expect(KillProcessIndex.search(records, query: "safari").first?.record.pid == 11)
}

@Test func killProcessIndexFormatsMemory() {
    #expect(KillProcessIndex.memoryDisplay(bytes: 512 * 1_048_576) == "512 MB")
    #expect(KillProcessIndex.memoryDisplay(bytes: 1536 * 1_048_576) == "1.5 GB")
    #expect(KillProcessIndex.memoryDisplay(bytes: nil) == "memory unknown")
}

@Test func killProcessHandleReturnsStaleCacheWithoutBlocking() async {
    let module = KillProcessModule()
    await module.seedCacheForTesting([
        RunningProcessRecord(pid: 10, bundleID: "com.apple.Preview", name: "Preview", launchDate: nil, residentBytes: nil)
    ])

    let query = Query(raw: "kill preview", sequence: 1, command: ParsedCommand(trigger: "kill", payload: "preview", module: .killProcess))
    let start = ContinuousClock.now
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1)))
    let elapsed = start.duration(to: .now)
    let elapsedMs = Double(elapsed.components.seconds) * 1000
        + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000

    #expect(result.items.first?.title == "Preview")
    #expect(elapsedMs < 50)
}

@Test func killProcessWarmupDoesNotAwaitFullRefresh() async {
    let module = KillProcessModule()
    let context = ModuleContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        database: NoopDatabaseClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: NoopConfigurationClient()
    )

    let start = ContinuousClock.now
    await module.warmup(context)
    let elapsed = start.duration(to: .now)
    let elapsedMs = Double(elapsed.components.seconds) * 1000
        + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000

    #expect(elapsedMs < 50)
    try? await Task.sleep(for: .milliseconds(200))
    #expect(await module.refreshCallCount >= 1)
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
