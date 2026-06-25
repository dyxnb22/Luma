import Foundation

public actor QueryDispatcher {
    private let host: ModuleHost
    private let usage: UsageTracking
    private let resultCache: UsageResultCache
    private let metrics: any MetricsClient

    public init(
        host: ModuleHost,
        usage: UsageTracking = InMemoryUsageTracker(),
        resultCache: UsageResultCache = UsageResultCache.defaultCache(),
        metrics: any MetricsClient = NoopMetricsClient()
    ) {
        self.host = host
        self.usage = usage
        self.resultCache = resultCache
        self.metrics = metrics
    }

    public func recentFrecency(limit: Int = 8) async -> [ResultItem] {
        let records = await usage.recent(limit: limit)
        let usageMap = await usage.snapshot()
        let query = Query(raw: "", sequence: 0)
        var items: [ResultItem] = []
        for record in records {
            guard let cached = await resultCache.item(for: record.id) else { continue }
            var ranked = cached
            ranked.rankingHints.finalScore = Ranker.score(item: cached, query: query, usage: usageMap[record.id])
            items.append(ranked)
        }
        return items.sorted { $0.rankingHints.finalScore > $1.rankingHints.finalScore }
    }

    public func dispatch(
        _ query: Query,
        onSnapshot: @Sendable @escaping (ResultSnapshot) async -> Void
    ) async {
        await metrics.mark("query.dispatch.start")
        let modules = await host.enabledQueryableModules()
        let usageRecords = await usage.snapshot()
        var merged: [ResultID: ScoredItem] = [:]

        await withTaskGroup(of: ModuleResult.self) { group in
            for module in modules {
                let manifest = type(of: module).manifest
                group.addTask {
                    await self.metrics.mark("module.\(manifest.identifier.rawValue).start")
                    let deadline = ContinuousClock().now.advanced(by: manifest.queryTimeout)
                    let context = QueryContext(deadline: deadline)
                    let result = await Timeout.run(after: manifest.queryTimeout) {
                        await module.handle(query, context: context)
                    } ?? ModuleResult.empty(
                        for: manifest.identifier,
                        diagnostic: ModuleDiagnostic(kind: .timeout, message: "Module timed out")
                    )
                    await self.metrics.mark("module.\(manifest.identifier.rawValue).finish")
                    return result
                }
            }

            for await result in group {
                guard !Task.isCancelled else { break }
                for item in result.items {
                    let usage = usageRecords[item.id]
                    var ranked = item
                    let score = Ranker.score(item: item, query: query, usage: usage)
                    ranked.rankingHints.fuzzyScore = FuzzyMatcher.score(query: query.normalized, target: item.title.lowercased())
                    ranked.rankingHints.finalScore = score
                    merged[item.id] = ScoredItem(item: ranked, score: score)
                }
                await onSnapshot(Self.snapshot(from: merged, sequence: query.sequence))
            }
        }
        await metrics.mark("query.dispatch.finish")
    }

    public func dispatchTargeted(
        _ query: Query,
        moduleID: ModuleIdentifier,
        onSnapshot: @Sendable @escaping (ResultSnapshot) async -> Void
    ) async {
        await metrics.mark("query.dispatch.targeted.start")
        guard let module = await host.module(moduleID) else {
            await onSnapshot(ResultSnapshot(querySequence: query.sequence, items: []))
            return
        }
        let manifest = type(of: module).manifest
        guard manifest.capabilities.contains(.queryable) else {
            await onSnapshot(ResultSnapshot(querySequence: query.sequence, items: []))
            return
        }

        let usageRecords = await usage.snapshot()
        let deadline = ContinuousClock().now.advanced(by: manifest.queryTimeout)
        let context = QueryContext(deadline: deadline)
        let result = await Timeout.run(after: manifest.queryTimeout) {
            await module.handle(query, context: context)
        } ?? ModuleResult.empty(
            for: manifest.identifier,
            diagnostic: ModuleDiagnostic(kind: .timeout, message: "Module timed out")
        )

        var merged: [ResultID: ScoredItem] = [:]
        for item in result.items {
            let usage = usageRecords[item.id]
            var ranked = item
            let score = Ranker.score(item: item, query: query, usage: usage)
            ranked.rankingHints.fuzzyScore = FuzzyMatcher.score(query: query.normalized, target: item.title.lowercased())
            ranked.rankingHints.finalScore = score
            merged[item.id] = ScoredItem(item: ranked, score: score)
        }
        await onSnapshot(Self.snapshot(from: merged, sequence: query.sequence))
        await metrics.mark("query.dispatch.targeted.finish")
    }

    private static func snapshot(from merged: [ResultID: ScoredItem], sequence: UInt64) -> ResultSnapshot {
        let items = merged.values
            .filter { $0.score > -.infinity }
            .sorted { $0.score > $1.score }
            .prefix(50)
            .map(\.item)
        return ResultSnapshot(querySequence: sequence, items: Array(items))
    }
}

private struct ScoredItem: Sendable {
    let item: ResultItem
    let score: Double
}

public protocol UsageTracking: Sendable {
    func snapshot() async -> [ResultID: UsageRecord]
    func record(_ id: ResultID, at date: Date) async
    func recent(limit: Int) async -> [UsageRecord]
}

public actor InMemoryUsageTracker: UsageTracking {
    private var records: [ResultID: UsageRecord] = [:]

    public init() {}

    public func snapshot() -> [ResultID: UsageRecord] {
        records
    }

    public func record(_ id: ResultID, at date: Date = Date()) {
        var record = records[id] ?? UsageRecord(id: id, count: 0, lastUsed: date)
        record.count += 1
        record.lastUsed = date
        records[id] = record
    }

    public func recent(limit: Int = 8) -> [UsageRecord] {
        Array(records.values.sorted { lhs, rhs in
            if lhs.lastUsed == rhs.lastUsed { return lhs.count > rhs.count }
            return lhs.lastUsed > rhs.lastUsed
        }.prefix(limit))
    }
}
