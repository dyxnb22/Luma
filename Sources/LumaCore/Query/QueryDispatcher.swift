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

    public func dispatch(
        _ query: Query,
        onSnapshot: @Sendable @escaping (ResultSnapshot) async -> Void
    ) async {
        await metrics.mark("query.dispatch.start")
        let modules = await host.enabledQueryableModules(forGlobalSearch: true)
        await metrics.mark("query.dispatch.fanout.\(modules.count)")
        let usageRecords = await usage.snapshot()
        var merged: [ResultID: ScoredItem] = [:]

        await withTaskGroup(of: (ModuleIdentifier, ModuleResult).self) { group in
            for module in modules {
                let manifest = type(of: module).manifest
                group.addTask {
                    await self.host.warmupIfNeeded(id: manifest.identifier, reason: .query)
                    await self.host.markUsed(id: manifest.identifier)
                    await self.metrics.mark("module.\(manifest.identifier.rawValue).start")
                    let deadline = ContinuousClock().now.advanced(by: manifest.queryTimeout)
                    let context = await self.host.makeQueryContext(deadline: deadline)
                    let result = await Timeout.run(after: manifest.queryTimeout) {
                        await module.handle(query, context: context)
                    } ?? ModuleResult.empty(
                        for: manifest.identifier,
                        diagnostic: ModuleDiagnostic(kind: .timeout, message: "Module timed out")
                    )
                    await self.metrics.mark("module.\(manifest.identifier.rawValue).finish")
                    return (manifest.identifier, result)
                }
            }

            for await (moduleID, result) in group {
                guard !Task.isCancelled else { break }
                Self.mergeItems(
                    from: result,
                    moduleID: moduleID,
                    query: query,
                    usageRecords: usageRecords,
                    keepModuleRowsWhenAllFilteredOut: false,
                    into: &merged
                )
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
        guard let module = await host.enabledModule(moduleID) else {
            let row = ModuleDiagnosticResults.informationalRow(
                module: moduleID,
                diagnostic: ModuleDiagnostic(
                    kind: .degraded,
                    message: "Module disabled in Settings — enable it to use this command"
                )
            )
            await onSnapshot(ResultSnapshot(querySequence: query.sequence, items: [row]))
            return
        }
        let manifest = type(of: module).manifest
        guard manifest.capabilities.contains(.queryable) else {
            await onSnapshot(ResultSnapshot(querySequence: query.sequence, items: []))
            return
        }

        let usageRecords = await usage.snapshot()
        await host.warmupIfNeeded(id: moduleID, reason: .query)
        await host.markUsed(id: moduleID)
        let deadline = ContinuousClock().now.advanced(by: manifest.queryTimeout)
        let context = await host.makeQueryContext(deadline: deadline)
        let result = await Timeout.run(after: manifest.queryTimeout) {
            await module.handle(query, context: context)
        } ?? ModuleResult.empty(
            for: manifest.identifier,
            diagnostic: ModuleDiagnostic(kind: .timeout, message: "Module timed out")
        )

        var merged: [ResultID: ScoredItem] = [:]
        Self.mergeItems(
            from: result,
            moduleID: moduleID,
            query: query,
            usageRecords: usageRecords,
            keepModuleRowsWhenAllFilteredOut: true,
            into: &merged
        )
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

    private static func mergeItems(
        from result: ModuleResult,
        moduleID: ModuleIdentifier,
        query: Query,
        usageRecords: [ResultID: UsageRecord],
        keepModuleRowsWhenAllFilteredOut: Bool,
        into merged: inout [ResultID: ScoredItem]
    ) {
        var scored: [ScoredItem] = []
        scored.reserveCapacity(result.items.count)
        for item in result.items {
            let usage = usageRecords[item.id]
            var ranked = item
            let score = Ranker.score(item: item, query: query, usage: usage)
            ranked.rankingHints.fuzzyScore = Ranker.fuzzyScore(
                query: query,
                target: item.title.lowercased(),
                secondary: item.subtitle?.lowercased()
            )
            ranked.rankingHints.finalScore = score
            scored.append(ScoredItem(item: ranked, score: score))
        }

        let surviving = scored.filter { $0.score > -.infinity }
        let rowsToMerge: [ScoredItem]
        if keepModuleRowsWhenAllFilteredOut, surviving.isEmpty, !result.items.isEmpty {
            // Targeted modules may return command rows (e.g. app top, s new) that do not
            // fuzzy-match the payload token. Keep module-provided rows instead of dropping all,
            // while preserving the module's original order. Global search must not use this
            // fallback, or non-matching rows would leak back into the hot-path result set.
            rowsToMerge = result.items.enumerated().map { index, item in
                var ranked = item
                ranked.rankingHints.fuzzyScore = 1.0
                let score = 1.0 - Double(index) * 0.001
                ranked.rankingHints.finalScore = score
                return ScoredItem(item: ranked, score: score)
            }
        } else {
            rowsToMerge = surviving
        }

        for row in rowsToMerge {
            merged[row.item.id] = row
        }
        if result.items.isEmpty, let diagnostic = result.diagnostic {
            let row = ModuleDiagnosticResults.informationalRow(module: moduleID, diagnostic: diagnostic)
            merged[row.id] = ScoredItem(item: row, score: 1_000)
        }
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
