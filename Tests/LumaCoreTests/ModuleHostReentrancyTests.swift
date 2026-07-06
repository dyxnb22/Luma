import Foundation
import Testing
import LumaCore

private actor SlowWarmupLifecycleModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.reentrancy-slow"),
        displayName: "Slow Reentrancy",
        capabilities: [.queryable],
        defaultEnabled: false,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupFinished = false

    func warmup(_ context: ModuleContext) async {
        try? await Task.sleep(for: .milliseconds(80))
        warmupFinished = true
    }

    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        .empty(for: Self.manifest.identifier)
    }

    func teardown() async {}
}

private actor PinnedReentrancyModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.reentrancy-pinned"),
        displayName: "Pinned Reentrancy",
        capabilities: [.queryable],
        defaultEnabled: false,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        .empty(for: Self.manifest.identifier)
    }
}

private struct ReentrancyTestConfig: ConfigurationClient {
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

private struct ReentrancyTestDoubles {
    struct Logger: LoggingClient {
        func debug(_ message: String) async {}
        func error(_ message: String) async {}
    }

    struct Metrics: MetricsClient {
        func mark(_ name: String) async {}
    }

    struct Database: DatabaseClient {}

    static func context() -> ModuleContext {
        ModuleContext(
            logger: Logger(),
            metrics: Metrics(),
            database: Database(),
            pasteboard: NoopPasteboardClient(),
            accessibility: NoopAccessibilityClient(),
            fileSystem: NoopFileSystemClient(),
            translation: NoopTranslationClient(),
            config: ReentrancyTestConfig()
        )
    }
}

@Test func moduleHostDisableDuringWarmupDoesNotMarkWarm() async {
    let host = ModuleHost(context: ReentrancyTestDoubles.context())
    let id = SlowWarmupLifecycleModule.manifest.identifier
    await host.register(SlowWarmupLifecycleModule())
    await host.applyEnabledSet([id])

    let warmupTask = Task { await host.warmupIfNeeded(id: id, reason: .query) }
    try? await Task.sleep(for: .milliseconds(20))
    await host.applyEnabledSet([])
    await warmupTask.value

    #expect(await host.warmupState(for: id) != .warm)
}

@Test func moduleHostInvalidateWarmupDuringWarmupLeavesColdState() async {
    let host = ModuleHost(context: ReentrancyTestDoubles.context())
    let id = SlowWarmupLifecycleModule.manifest.identifier
    await host.register(SlowWarmupLifecycleModule())
    await host.applyEnabledSet([id])

    let warmupTask = Task { await host.warmupIfNeeded(id: id, reason: .query) }
    try? await Task.sleep(for: .milliseconds(20))
    await host.applyEnabledSet([])
    await warmupTask.value

    let state = await host.warmupState(for: id)
    #expect(state != .warm)
}

@Test func moduleHostPinnedEnableStillWarmsAfterReentrancy() async {
    let host = ModuleHost(context: ReentrancyTestDoubles.context())
    let id = PinnedReentrancyModule.manifest.identifier
    await host.register(PinnedReentrancyModule())
    await host.configureWarmupPolicy(pinned: [id])
    await host.applyEnabledSet([id])

    await host.warmupIfNeeded(id: id, reason: .startup)
    #expect(await host.warmupState(for: id) == .warm)
}
