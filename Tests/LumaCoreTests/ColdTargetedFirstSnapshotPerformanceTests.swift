import Foundation
import LumaCore
import LumaModules
import Testing

private func p95(_ samples: [Double]) -> Double {
    let sorted = samples.sorted()
    return sorted[Int(Double(sorted.count - 1) * 0.95)]
}

private actor SlowWarmProjectsStyleModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.cold-first"),
        displayName: "Cold First",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(400)
    )

    func warmup(_ context: ModuleContext) async {
        try? await Task.sleep(for: .milliseconds(200))
    }

    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        ModuleResult(items: [
            ResultItem(
                id: ResultID(module: Self.manifest.identifier, key: "one"),
                title: "Ready",
                titleAttributed: AttributedString("Ready"),
                icon: .none,
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "run"),
                    title: "Run",
                    kind: .noop
                ),
                rankingHints: RankingHints()
            )
        ])
    }
}

private actor FirstSnapshotCollector {
    private(set) var firstSnapshotMilliseconds: Double?

    func recordIfFirst(start: ContinuousClock.Instant) {
        guard firstSnapshotMilliseconds == nil else { return }
        let elapsed = start.duration(to: .now)
        firstSnapshotMilliseconds = Double(elapsed.components.seconds) * 1000
            + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
    }
}

private struct TestDatabase: DatabaseClient {}

private struct TestConfig: ConfigurationClient {
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

private func moduleContext() -> ModuleContext {
    ModuleContext(
        logger: TestLogger(),
        metrics: NoopMetricsClient(),
        database: TestDatabase(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: TestConfig()
    )
}

private struct TestLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

@Test func coldTargetedFirstSnapshotStaysUnderBudget() async {
    let parsed = ParsedCommand(
        trigger: "cold",
        payload: "first",
        module: SlowWarmProjectsStyleModule.manifest.identifier
    )
    let query = Query(
        raw: "cold first",
        sequence: 1,
        command: parsed
    )

    var samples: [Double] = []
    for iteration in 0..<20 {
        let host = ModuleHost(context: moduleContext())
        await host.register(SlowWarmProjectsStyleModule())
        let dispatcher = QueryDispatcher(host: host)
        let collector = FirstSnapshotCollector()
        let start = ContinuousClock.now
        await dispatcher.dispatchTargeted(query, moduleID: SlowWarmProjectsStyleModule.manifest.identifier) { _ in
            await collector.recordIfFirst(start: start)
        }
        if let ms = await collector.firstSnapshotMilliseconds {
            samples.append(ms)
        }
        _ = iteration
    }

    #expect(p95(samples) < 30)
}
