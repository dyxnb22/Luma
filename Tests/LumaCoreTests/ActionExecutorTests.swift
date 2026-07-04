import Foundation
import Testing
import LumaCore

private actor FailingActionModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.failing-action"),
        displayName: "Failing",
        capabilities: [.providesActions],
        defaultEnabled: true,
        priority: 0,
        queryTimeout: .milliseconds(20)
    )

    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        .empty(for: Self.manifest.identifier)
    }

    func perform(_ action: Action, context: ActionContext) async throws {
        throw ModuleError.dataUnavailable
    }
}

private actor RecordingPasteboard: PasteboardClient {
    private(set) var writes: [String] = []

    func write(_ string: String) async { writes.append(string) }
    func writeSecure(_ string: String, clearAfterSeconds: Int) async { writes.append(string) }
    func writeImage(data: Data, pasteboardType: String) async {}
    func writeFileURLs(_ urls: [URL]) async {}
    func readString() async -> String? { nil }
}

private struct FailingTranslationClient: TranslationClient {
    struct TestError: Error {}

    func translate(_ text: String) async throws -> TranslationOutcome {
        _ = text
        throw TestError()
    }
}

private struct ActionExecutorTestLogger: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private func makeExecutor(
    host: ModuleHost,
    pasteboard: any PasteboardClient = RecordingPasteboard(),
    translation: any TranslationClient = NoopTranslationClient()
) -> ActionExecutor {
    let usage = InMemoryUsageTracker()
    let context = ActionContext(
        logger: ActionExecutorTestLogger(),
        metrics: NoopMetricsClient(),
        pasteboard: pasteboard,
        accessibility: NoopAccessibilityClient(),
        translation: translation
    )
    return ActionExecutor(host: host, context: context, usage: usage, resultCache: UsageResultCache(url: temporaryCacheURL()))
}

private func temporaryCacheURL() -> URL {
    FileManager.default.temporaryDirectory.appendingPathComponent("action-cache-\(UUID().uuidString).json")
}

@Test func actionExecutorSuccessRecordsUsageAndCache() async throws {
    let host = ModuleHost(context: ModuleContext(logger: ActionExecutorTestLogger(), metrics: NoopMetricsClient(), database: NoopDatabaseClient(), pasteboard: NoopPasteboardClient(), accessibility: NoopAccessibilityClient(), fileSystem: NoopFileSystemClient(), translation: NoopTranslationClient(), config: NoopConfigurationClient()))
    let pasteboard = RecordingPasteboard()
    let usage = InMemoryUsageTracker()
    let cacheURL = temporaryCacheURL()
    defer { try? FileManager.default.removeItem(at: cacheURL) }
    let cache = UsageResultCache(url: cacheURL)
    let executor = ActionExecutor(
        host: host,
        context: ActionContext(
            logger: ActionExecutorTestLogger(),
            metrics: NoopMetricsClient(),
            pasteboard: pasteboard,
            accessibility: NoopAccessibilityClient()
        ),
        usage: usage,
        resultCache: cache
    )

    let item = ResultItem(
        id: ResultID(module: .apps, key: "test"),
        title: "Test",
        titleAttributed: AttributedString("Test"),
        icon: .symbol("app"),
        primaryAction: Action(
            id: ActionID(module: .apps, key: "copy"),
            title: "Copy",
            kind: .copyToPasteboard("hello")
        ),
        rankingHints: RankingHints()
    )

    let result = await executor.run(item.primaryAction, for: item)
    #expect(result == .success)
    #expect(await pasteboard.writes == ["hello"])
    #expect(await usage.snapshot()[item.id]?.count == 1)
    #expect(await cache.item(for: item.id)?.title == "Test")
}

@Test func actionExecutorFailureSkipsUsageAndCache() async throws {
    let host = ModuleHost(context: ModuleContext(logger: ActionExecutorTestLogger(), metrics: NoopMetricsClient(), database: NoopDatabaseClient(), pasteboard: NoopPasteboardClient(), accessibility: NoopAccessibilityClient(), fileSystem: NoopFileSystemClient(), translation: NoopTranslationClient(), config: NoopConfigurationClient()))
    await host.register(FailingActionModule())
    let usage = InMemoryUsageTracker()
    let cacheURL = temporaryCacheURL()
    defer { try? FileManager.default.removeItem(at: cacheURL) }
    let cache = UsageResultCache(url: cacheURL)
    let executor = ActionExecutor(
        host: host,
        context: ActionContext(
            logger: ActionExecutorTestLogger(),
            metrics: NoopMetricsClient(),
            pasteboard: NoopPasteboardClient(),
            accessibility: NoopAccessibilityClient()
        ),
        usage: usage,
        resultCache: cache
    )

    let moduleID = FailingActionModule.manifest.identifier
    let item = ResultItem(
        id: ResultID(module: moduleID, key: "fail"),
        title: "Fail",
        titleAttributed: AttributedString("Fail"),
        icon: .symbol("xmark"),
        primaryAction: Action(
            id: ActionID(module: moduleID, key: "fail"),
            title: "Fail",
            kind: .custom(payload: Data(), handler: moduleID)
        ),
        rankingHints: RankingHints()
    )

    let result = await executor.run(item.primaryAction, for: item)
    #expect(result == .failure(message: "Required data is unavailable.", recoverable: true))
    #expect(await usage.snapshot().isEmpty)
    #expect(await cache.item(for: item.id) == nil)
}

@Test func actionExecutorNoopSkipsUsageAndCache() async throws {
    let host = ModuleHost(context: ModuleContext(logger: ActionExecutorTestLogger(), metrics: NoopMetricsClient(), database: NoopDatabaseClient(), pasteboard: NoopPasteboardClient(), accessibility: NoopAccessibilityClient(), fileSystem: NoopFileSystemClient(), translation: NoopTranslationClient(), config: NoopConfigurationClient()))
    let usage = InMemoryUsageTracker()
    let cacheURL = temporaryCacheURL()
    defer { try? FileManager.default.removeItem(at: cacheURL) }
    let cache = UsageResultCache(url: cacheURL)
    let executor = ActionExecutor(
        host: host,
        context: ActionContext(
            logger: ActionExecutorTestLogger(),
            metrics: NoopMetricsClient(),
            pasteboard: NoopPasteboardClient(),
            accessibility: NoopAccessibilityClient()
        ),
        usage: usage,
        resultCache: cache
    )

    let item = ResultItem(
        id: ResultID(module: .apps, key: "noop"),
        title: "Noop",
        titleAttributed: AttributedString("Noop"),
        icon: .symbol("app"),
        primaryAction: Action(id: ActionID(module: .apps, key: "noop"), title: "Noop", kind: .noop),
        rankingHints: RankingHints()
    )

    let result = await executor.run(item.primaryAction, for: item)
    #expect(result == .success)
    #expect(await usage.snapshot().isEmpty)
    #expect(await cache.item(for: item.id) == nil)
}

@Test func actionExecutorTranslationFailureReturnsMappedMessage() async {
    let host = ModuleHost(context: ModuleContext(logger: ActionExecutorTestLogger(), metrics: NoopMetricsClient(), database: NoopDatabaseClient(), pasteboard: NoopPasteboardClient(), accessibility: NoopAccessibilityClient(), fileSystem: NoopFileSystemClient(), translation: FailingTranslationClient(), config: NoopConfigurationClient()))
    let executor = makeExecutor(host: host, translation: FailingTranslationClient())
    let id = ResultID(module: .translate, key: "tr")
    let result = await executor.run(
        Action(id: ActionID(module: .translate, key: "tr"), title: "Translate", kind: .translateText("hi")),
        for: id
    )
    #expect(result.succeeded == false)
    #expect(result.userFacingMessage == nil)
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

private struct NoopFileSystemClient: FileSystemClient {
    func watch(root: URL, debounceMillis: Int) async -> AsyncStream<[FSChangeEvent]> {
        AsyncStream { $0.finish() }
    }

    func stopWatching(root: URL) async {}
}
