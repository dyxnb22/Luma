import Foundation
import LumaCore
import Testing
@testable import LumaModules

@Test func wordbookModuleReviewUsesCachedDueCount() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let module = WordbookModule(store: store)
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

    let parsed = ParsedCommand(trigger: "word", payload: "review", module: .wordbook)
    _ = await module.handle(
        Query(raw: "word review", sequence: 1, command: parsed),
        context: QueryContext(deadline: .now + .seconds(1))
    )
    _ = await module.handle(
        Query(raw: "word review", sequence: 2, command: parsed),
        context: QueryContext(deadline: .now + .seconds(1))
    )

    #expect(await module.dueTodayCountQueryCount == 0)
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

private func p95(_ samples: [Double]) -> Double {
    let sorted = samples.sorted()
    return sorted[Int(Double(sorted.count - 1) * 0.95)]
}

@Test func wordbookLargeIndexSearchStaysUnderBudget() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }
    try WordbookTestFixtures.bulkInsertWords(at: url, count: 5_000, prefix: "lemma")

    let module = WordbookModule(store: store)
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

    let parsed = ParsedCommand(trigger: "word", payload: "lemma42", module: .wordbook)
    let query = Query(raw: "word lemma42", sequence: 1, command: parsed)
    var samples: [Double] = []
    for sequence in 0..<100 {
        let start = ContinuousClock.now
        _ = await module.handle(
            Query(raw: query.raw, sequence: UInt64(sequence), command: parsed),
            context: QueryContext(deadline: .now + .seconds(1))
        )
        let elapsed = start.duration(to: .now)
        samples.append(
            Double(elapsed.components.seconds) * 1000
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        )
    }

    #expect(p95(samples) < 30)
}
