import Foundation
import Testing
import LumaCore
import LumaModules

private actor HotPathCountingModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.hot"),
        displayName: "Hot",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult { .empty(for: Self.manifest.identifier) }
}

private actor OnDemandCountingModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.ondemand"),
        displayName: "OnDemand",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupCount = 0
    private(set) var handleCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        handleCount += 1
        return .empty(for: Self.manifest.identifier)
    }
}

private struct TierTestConfig: ConfigurationClient {
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

private struct TierTestDoubles {
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
            config: TierTestConfig()
        )
    }
}

@Test func globalDispatchSkipsWarmupForNonGlobalSearchModules() async {
    let host = ModuleHost(context: TierTestDoubles.context())
    let hot = HotPathCountingModule()
    let onDemand = OnDemandCountingModule()
    await host.register(hot)
    await host.register(onDemand)
    await host.applyEnabledSet([
        HotPathCountingModule.manifest.identifier,
        OnDemandCountingModule.manifest.identifier
    ])
    await host.configureGlobalSearchModuleIDs([HotPathCountingModule.manifest.identifier])

    let dispatcher = QueryDispatcher(host: host)
    let query = Query(raw: "hello", sequence: 1)
    await dispatcher.dispatch(query) { _ in }

    #expect(await hot.warmupCount == 1)
    #expect(await onDemand.warmupCount == 0)
}

@Test func targetedDispatchWarmsOnDemandModule() async {
    let host = ModuleHost(context: TierTestDoubles.context())
    let onDemand = OnDemandCountingModule()
    await host.register(onDemand)
    await host.applyEnabledSet([OnDemandCountingModule.manifest.identifier])
    await host.configureGlobalSearchModuleIDs([ModuleIdentifier(rawValue: "luma.test.hot")])

    let dispatcher = QueryDispatcher(host: host)
    let query = Query(raw: "n daily", sequence: 1)
    await dispatcher.dispatchTargeted(query, moduleID: OnDemandCountingModule.manifest.identifier) { _ in }

    #expect(await onDemand.warmupCount == 1)
}

@Test func targetedDispatchSkipsDisabledModule() async {
    let host = ModuleHost(context: TierTestDoubles.context())
    let onDemand = OnDemandCountingModule()
    await host.register(onDemand)
    await host.applyEnabledSet([])

    let dispatcher = QueryDispatcher(host: host)
    let query = Query(raw: "n daily", sequence: 1)
    await dispatcher.dispatchTargeted(query, moduleID: OnDemandCountingModule.manifest.identifier) { _ in }

    #expect(await onDemand.warmupCount == 0)
    #expect(await onDemand.handleCount == 0)
}

@Test func moduleRegistryGlobalSearchMatchesHotPathTier() {
    let global = ModuleRegistry.globalSearchModuleIDs
    let hot = ModuleRegistry.hotPathModuleIDs
    #expect(global == hot)
    #expect(global.contains(ModuleIdentifier(rawValue: "luma.clipboard")))
    #expect(!global.contains(ModuleIdentifier(rawValue: "luma.notes")))
}
