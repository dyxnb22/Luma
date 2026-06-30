import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

// Fast-path global search replay (default module set; Browser Tabs stays default-off).
// Slow targeted modules: see SlowModuleQueryPerformanceTests.swift
@Test func appSearchThousandKeystrokeReplayStaysUnderBudget() async {
    let logger = LumaLogger()
    let metrics = LumaMetrics()
    let context = ModuleContext(
        logger: logger,
        metrics: metrics,
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(),
        config: ConfigurationStore()
    )
    let host = ModuleHost(context: context)
    for module in BuiltInModules.makeAll() {
        await host.register(module)
    }
    await host.warmupAll()

    let usage = InMemoryUsageTracker()
    let dispatcher = QueryDispatcher(host: host, usage: usage)
    let queries = (0..<1000).map { $0.isMultiple(of: 2) ? "saf" : "app \($0 % 100)" }
    var samples: [Double] = []

    for query in queries {
        let start = ContinuousClock.now
        await dispatcher.dispatch(Query(raw: query, sequence: 0)) { _ in }
        let elapsed = start.duration(to: .now)
        samples.append(Double(elapsed.components.seconds) * 1000 + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000)
    }

    let sorted = samples.sorted()
    let p95 = sorted[Int(Double(sorted.count - 1) * 0.95)]
    #expect(p95 < 30)
}

@Test func appSearchColdPinnedWarmupReplayStaysUnderBudget() async {
    let logger = LumaLogger()
    let metrics = LumaMetrics()
    let config = ConfigurationStore()
    let context = ModuleContext(
        logger: logger,
        metrics: metrics,
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(config: config),
        config: config
    )
    let host = ModuleHost(context: context)
    let modules = BuiltInModules.makeAll()
    for module in modules {
        await host.register(module)
    }
    let pinned = ModuleWarmupDefaults.defaultPinnedModuleIDs
    let enabled = Set(modules.map { type(of: $0).manifest.identifier })
    await config.setPinnedModuleIDs(pinned)
    await config.setEnabledModules(enabled)
    await host.configureGlobalSearchModuleIDs(ModuleRegistry.globalSearchModuleIDs)
    await host.configureWarmupPolicy(pinned: pinned)
    await host.applyEnabledSet(enabled)
    await host.warmupIfNeeded(ids: pinned.intersection(enabled), reason: .startup)

    let usage = InMemoryUsageTracker()
    let dispatcher = QueryDispatcher(host: host, usage: usage)
    let queries = (0..<200).map { $0.isMultiple(of: 2) ? "saf" : "clip \($0 % 50)" }

    // Discard first replay batch so measurement excludes one-off JIT / lazy init spikes.
    for query in queries.prefix(40) {
        await dispatcher.dispatch(Query(raw: query, sequence: 0)) { _ in }
    }

    var samples: [Double] = []
    for query in queries {
        let start = ContinuousClock.now
        await dispatcher.dispatch(Query(raw: query, sequence: 0)) { _ in }
        let elapsed = start.duration(to: .now)
        samples.append(Double(elapsed.components.seconds) * 1000 + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000)
    }

    let sorted = samples.sorted()
    let p95 = sorted[Int(Double(sorted.count - 1) * 0.95)]
    #expect(p95 < 30)
}
