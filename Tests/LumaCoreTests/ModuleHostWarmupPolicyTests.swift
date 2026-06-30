import Foundation
import Testing
import LumaCore

private actor CountingLifecycleModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.lifecycle"),
        displayName: "Test",
        capabilities: [.queryable],
        defaultEnabled: false,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupCount = 0
    private(set) var teardownCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult { .empty(for: Self.manifest.identifier) }
    func teardown() async { teardownCount += 1 }
}

private actor PinnedCountingLifecycleModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.pinned-lifecycle"),
        displayName: "Pinned Test",
        capabilities: [.queryable],
        defaultEnabled: false,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupCount = 0
    private(set) var teardownCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult { .empty(for: Self.manifest.identifier) }
    func teardown() async { teardownCount += 1 }
}

private struct WarmupTestConfig: ConfigurationClient {
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

private struct WarmupTestDoubles {
    struct Logger: LoggingClient {
        func debug(_ message: String) async {}
        func error(_ message: String) async {}
    }

    struct Database: DatabaseClient {}
    struct Pasteboard: PasteboardClient {
        func write(_ string: String) async {}
        func writeSecure(_ string: String, clearAfterSeconds: Int) async {}
        func writeImage(data: Data, pasteboardType: String) async {}
        func writeFileURLs(_ urls: [URL]) async {}
        func readString() async -> String? { nil }
    }

    struct Accessibility: AccessibilityClient {
        func isTrusted() async -> Bool { false }
        func requestPermission() async {}
        func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
        func insert(text: String) async {}
        func applyWindowLayout(_ preset: String) async {}
    }

    struct FileSystem: FileSystemClient {
        func watch(root: URL, debounceMillis: Int) async -> AsyncStream<[FSChangeEvent]> {
            AsyncStream { $0.finish() }
        }
        func stopWatching(root: URL) async {}
    }

    struct Translation: TranslationClient {
        func translate(_ text: String) async throws -> TranslationOutcome {
            TranslationOutcome(text: text)
        }
    }

    static func context() -> ModuleContext {
        ModuleContext(
            logger: Logger(),
            metrics: NoopMetricsClient(),
            database: Database(),
            pasteboard: Pasteboard(),
            accessibility: Accessibility(),
            fileSystem: FileSystem(),
            translation: Translation(),
            config: WarmupTestConfig()
        )
    }
}

@Test func moduleHostWarmupIfNeededSkipsWhenNotPinnedOnEnable() async {
    let host = ModuleHost(context: WarmupTestDoubles.context())
    let module = CountingLifecycleModule()
    await host.register(module)
    await host.configureWarmupPolicy(pinned: [])
    await host.applyEnabledSet([CountingLifecycleModule.manifest.identifier])
    #expect(await module.warmupCount == 0)
}

@Test func moduleHostWarmupIfNeededWarmsOnQuery() async {
    let host = ModuleHost(context: WarmupTestDoubles.context())
    let module = CountingLifecycleModule()
    await host.register(module)
    await host.applyEnabledSet([CountingLifecycleModule.manifest.identifier])
    await host.warmupIfNeeded(id: CountingLifecycleModule.manifest.identifier, reason: .query)
    #expect(await module.warmupCount == 1)
    #expect(await host.warmupState(for: CountingLifecycleModule.manifest.identifier) == .warm)
}

@Test func moduleHostTeardownIdleModulesSkipsReservedModules() async {
    let host = ModuleHost(context: WarmupTestDoubles.context())
    let module = CountingLifecycleModule()
    await host.register(module)
    await host.applyEnabledSet([CountingLifecycleModule.manifest.identifier])
    await host.warmupIfNeeded(id: CountingLifecycleModule.manifest.identifier, reason: .query)
    await host.setReservedModuleIDs([CountingLifecycleModule.manifest.identifier])

    await host.teardownIdleModules(olderThan: .zero, pinned: [])

    #expect(await module.teardownCount == 0)
    #expect(await host.warmupState(for: CountingLifecycleModule.manifest.identifier) == .warm)
}

@Test func moduleHostTeardownAfterReserveClear() async {
    let host = ModuleHost(context: WarmupTestDoubles.context())
    let module = CountingLifecycleModule()
    await host.register(module)
    await host.applyEnabledSet([CountingLifecycleModule.manifest.identifier])
    await host.warmupIfNeeded(id: CountingLifecycleModule.manifest.identifier, reason: .query)
    await host.setReservedModuleIDs([CountingLifecycleModule.manifest.identifier])
    await host.setReservedModuleIDs([])

    await host.teardownIdleModules(olderThan: .zero, pinned: [], reason: .idle)

    #expect(await module.teardownCount == 1)
    #expect(await host.warmupState(for: CountingLifecycleModule.manifest.identifier) == .tornDown)
}

@Test func moduleHostDebugSnapshotReportsWarmAndReserved() async {
    let host = ModuleHost(context: WarmupTestDoubles.context())
    let module = CountingLifecycleModule()
    await host.register(module)
    await host.applyEnabledSet([CountingLifecycleModule.manifest.identifier])
    await host.warmupIfNeeded(id: CountingLifecycleModule.manifest.identifier, reason: .query)
    await host.setReservedModuleIDs([CountingLifecycleModule.manifest.identifier])

    let snapshot = await host.debugSnapshot()
    #expect(snapshot.warmModuleIDs.contains(CountingLifecycleModule.manifest.identifier))
    #expect(snapshot.reservedModuleIDs.contains(CountingLifecycleModule.manifest.identifier))
}

@Test func moduleHostTeardownIdleModulesSkipsPinnedModules() async {
    let host = ModuleHost(context: WarmupTestDoubles.context())
    let idle = CountingLifecycleModule()
    let pinned = PinnedCountingLifecycleModule()
    await host.register(idle)
    await host.register(pinned)
    await host.applyEnabledSet([
        CountingLifecycleModule.manifest.identifier,
        PinnedCountingLifecycleModule.manifest.identifier
    ])
    await host.warmupIfNeeded(ids: [
        CountingLifecycleModule.manifest.identifier,
        PinnedCountingLifecycleModule.manifest.identifier
    ], reason: .query)

    await host.teardownIdleModules(
        olderThan: .zero,
        pinned: [PinnedCountingLifecycleModule.manifest.identifier]
    )

    #expect(await idle.teardownCount == 1)
    #expect(await pinned.teardownCount == 0)
    #expect(await host.warmupState(for: CountingLifecycleModule.manifest.identifier) == .tornDown)
    #expect(await host.warmupState(for: PinnedCountingLifecycleModule.manifest.identifier) == .warm)
}
