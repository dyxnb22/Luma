import Foundation
import Testing
import LumaCore
import LumaModules

private struct TestDoubles {
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
            _ = root
            _ = debounceMillis
            return AsyncStream { $0.finish() }
        }
        func stopWatching(root: URL) async { _ = root }
    }

    struct Translation: TranslationClient {
        func translate(_ text: String) async throws -> TranslationOutcome {
            TranslationOutcome(text: text)
        }
    }

    struct Config: ConfigurationClient {
        func enabledModules() async -> Set<ModuleIdentifier>? { nil }
        func clipboardMaxEntries() async -> Int { 100 }
        func clipboardMaxAgeDays() async -> Int { 30 }
        func clipboardMaxEntrySizeKB() async -> Int { 512 }
        func clipboardHistoryEnabled() async -> Bool { true }
        func clipboardIgnoredBundleIDs() async -> [String] { [] }
        func clipboardPasteBehavior() async -> String { "pasteDirectly" }
        func translationTargetLanguage() async -> String { "en" }
        func secretsAutoClearSeconds() async -> Int { 30 }
        func secretsRelockTimeoutSeconds() async -> Int { 300 }
        func secretsRequireUnlockOnLaunch() async -> Bool { false }
    }

    static func context(processMemory: any ProcessMemoryClient = NoopProcessMemoryClient()) -> ModuleContext {
        ModuleContext(
            logger: Logger(),
            metrics: NoopMetricsClient(),
            database: Database(),
            pasteboard: Pasteboard(),
            accessibility: Accessibility(),
            fileSystem: FileSystem(),
            translation: Translation(),
            config: Config(),
            processMemory: processMemory
        )
    }
}

private actor AlphaModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.alpha"),
        displayName: "Alpha",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(20)
    )

    func warmup(_ context: ModuleContext) async {}

    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        ModuleResult(items: [row(title: "Alpha \(query.raw)", module: Self.manifest.identifier)])
    }
}

private actor BetaModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.beta"),
        displayName: "Beta",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(20)
    )

    func warmup(_ context: ModuleContext) async {}

    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        ModuleResult(items: [row(title: "Beta \(query.raw)", module: Self.manifest.identifier)])
    }
}

private actor DiagnosticModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.diagnostic"),
        displayName: "Diagnostic",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(20)
    )

    func warmup(_ context: ModuleContext) async {}

    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        ModuleResult(
            items: [],
            diagnostic: ModuleDiagnostic(kind: .permissionRequired, message: "Automation denied for Safari")
        )
    }
}

private actor AlphaLifecycleModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.lifecycle.alpha"),
        displayName: "Alpha Lifecycle",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(20)
    )

    private(set) var warmupCount = 0
    private(set) var teardownCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult { ModuleResult(items: []) }
    func teardown() async { teardownCount += 1 }
}

private actor BetaLifecycleModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.lifecycle.beta"),
        displayName: "Beta Lifecycle",
        capabilities: [.queryable],
        defaultEnabled: false,
        priority: 1,
        queryTimeout: .milliseconds(20)
    )

    private(set) var warmupCount = 0
    private(set) var teardownCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult { ModuleResult(items: []) }
    func teardown() async { teardownCount += 1 }
}

private func row(title: String, module: ModuleIdentifier) -> ResultItem {
    ResultItem(
        id: ResultID(module: module, key: title),
        title: title,
        titleAttributed: AttributedString(title),
        icon: .none,
        primaryAction: Action(
            id: ActionID(module: module, key: "run"),
            title: "Run",
            kind: .noop
        ),
        rankingHints: RankingHints()
    )
}

private actor SnapshotCollector {
    private(set) var values: [ResultSnapshot] = []

    func append(_ snapshot: ResultSnapshot) {
        values.append(snapshot)
    }
}

@Test func targetedDispatchOnlyCallsSelectedModule() async {
    let host = ModuleHost(context: TestDoubles.context())
    await host.register(AlphaModule())
    await host.register(BetaModule())
    let dispatcher = QueryDispatcher(host: host)

    let query = Query(raw: "rec test", sequence: 1)
    let collector = SnapshotCollector()
    await dispatcher.dispatchTargeted(query, moduleID: AlphaModule.manifest.identifier) { snapshot in
        await collector.append(snapshot)
    }

    let snapshots = await collector.values
    #expect(snapshots.count == 1)
    #expect(snapshots[0].items.count == 1)
    #expect(snapshots[0].items[0].title == "Alpha rec test")
    #expect(snapshots[0].items[0].id.module == AlphaModule.manifest.identifier)
}

@Test func globalDispatchFansOutToModules() async {
    let host = ModuleHost(context: TestDoubles.context())
    await host.register(AlphaModule())
    await host.register(BetaModule())
    let dispatcher = QueryDispatcher(host: host)

    let query = Query(raw: "hello", sequence: 2)
    let collector = SnapshotCollector()
    await dispatcher.dispatch(query) { snapshot in
        await collector.append(snapshot)
    }

    let snapshots = await collector.values
    let titles = Set(snapshots.last?.items.map(\.title) ?? [])
    #expect(titles.contains("Alpha hello"))
    #expect(titles.contains("Beta hello"))
}

@Test func targetedDispatchSurfacesModuleDiagnosticAsInformationalRow() async {
    let host = ModuleHost(context: TestDoubles.context())
    await host.register(DiagnosticModule())
    let dispatcher = QueryDispatcher(host: host)

    let query = Query(raw: "tab github", sequence: 3)
    let collector = SnapshotCollector()
    await dispatcher.dispatchTargeted(query, moduleID: DiagnosticModule.manifest.identifier) { snapshot in
        await collector.append(snapshot)
    }

    let snapshots = await collector.values
    #expect(snapshots.count == 1)
    #expect(snapshots[0].items.count == 1)
    #expect(snapshots[0].items[0].rowKind == .informational)
    #expect(snapshots[0].items[0].title == "Automation denied for Safari")
    #expect(snapshots[0].items[0].subtitle == "Permission required")
}

@Test func targetedDispatchKeepsAppTopMemoryRowsDespitePayloadMismatch() async {
    let host = ModuleHost(context: TestDoubles.context(processMemory: StubProcessMemoryClient()))
    await host.register(AppsModule())
    let dispatcher = QueryDispatcher(host: host)

    let parsed = ParsedCommand(trigger: "app", payload: "top", module: .apps)
    let query = Query(raw: "app top", sequence: 4, command: parsed)
    let collector = SnapshotCollector()
    await dispatcher.dispatchTargeted(query, moduleID: .apps) { snapshot in
        await collector.append(snapshot)
    }

    let snapshots = await collector.values
    #expect(snapshots.count == 1)
    #expect(snapshots[0].items.count == 1)
    #expect(snapshots[0].items[0].title == "Safari")
    #expect(snapshots[0].items[0].subtitle == "512 MB")
}

@Test func targetedDispatchKeepsSnippetCreateRowForNewCommand() async {
    let host = ModuleHost(context: TestDoubles.context())
    await host.register(SnippetsModule())
    let dispatcher = QueryDispatcher(host: host)

    let parsed = ParsedCommand(trigger: "s", payload: "new", module: .snippets)
    let query = Query(raw: "s new", sequence: 5, command: parsed)
    let collector = SnapshotCollector()
    await dispatcher.dispatchTargeted(query, moduleID: .snippets) { snapshot in
        await collector.append(snapshot)
    }

    let snapshots = await collector.values
    #expect(snapshots.count == 1)
    #expect(snapshots[0].items.count == 1)
    #expect(snapshots[0].items[0].title == "Create Snippet")
    #expect(snapshots[0].items[0].subtitle == "Untitled")
}

private struct StubProcessMemoryClient: ProcessMemoryClient {
    func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        _ = limit
        return [
            RunningApplicationMemory(
                bundleID: "com.apple.Safari",
                name: "Safari",
                residentBytes: 512 * 1_048_576
            )
        ]
    }
}

@Test func moduleHostApplyEnabledSetWarmupsAddedAndTearsDownRemoved() async {
    let host = ModuleHost(context: TestDoubles.context())
    let alpha = AlphaLifecycleModule()
    let beta = BetaLifecycleModule()
    await host.register(alpha)
    await host.register(beta)
    await host.configureWarmupPolicy(pinned: [BetaLifecycleModule.manifest.identifier])

    await host.applyEnabledSet([AlphaLifecycleModule.manifest.identifier])
    await host.applyEnabledSet([
        AlphaLifecycleModule.manifest.identifier,
        BetaLifecycleModule.manifest.identifier,
    ])
    await host.applyEnabledSet([BetaLifecycleModule.manifest.identifier])

    #expect(await alpha.warmupCount == 0)
    #expect(await beta.warmupCount == 1)
    #expect(await alpha.teardownCount == 1)
    #expect(await beta.teardownCount == 0)
}

@Test func commandEntryUnknownPrefixBuildsCorrectionRow() {
    let suggestion = CommandSuggestion(
        trigger: "win",
        title: "Move Window",
        module: ModuleIdentifier(rawValue: "luma.window-layouts")
    )
    let rows = CommandEntryResults.unknownPrefixRows(
        prefix: "wni",
        suggestions: [suggestion],
        remainder: "left"
    )
    #expect(rows.first?.subtitle == "Did you mean \"win\"?")
    #expect(rows.first?.id.key == "replace:win left")
}

@Test func commandEntryGlobalHelpListsDiscoverableCommands() {
    let rows = CommandEntryResults.globalHelp(registry: BuiltInCommandRegistry.make())
    let commandRows = rows.filter { $0.id.key.hasPrefix("help.") && $0.id.key != "help.footer" }
    let triggers = commandRows.compactMap { row -> String? in
        row.title.split(separator: " ", maxSplits: 1).first.map(String.init)
    }
    #expect(triggers.contains("p"))
    #expect(triggers.contains("win"))
    #expect(triggers.contains("tr"))
    #expect(triggers.contains("rec"))
    #expect(rows.last?.id.key == "help.footer")
}
