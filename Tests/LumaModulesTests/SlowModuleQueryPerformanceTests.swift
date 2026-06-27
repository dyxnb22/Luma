import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private func p95(_ samples: [Double]) -> Double {
    let sorted = samples.sorted()
    return sorted[Int(Double(sorted.count - 1) * 0.95)]
}

private func moduleContext() -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(),
        config: ConfigurationStore()
    )
}

private func targetedQuery(raw: String) -> Query {
    let router = CommandRouter()
    let route = router.route(raw: raw)
    return Query(
        raw: raw,
        sequence: 0,
        command: router.registry.parsedCommand(for: raw, route: route)
    )
}

@Test func slowModuleKillTargetedQueryStaysUnderBudget() async {
    let host = ModuleHost(context: moduleContext())
    await host.register(KillProcessModule())
    let dispatcher = QueryDispatcher(host: host)
    let query = targetedQuery(raw: "kill preview")

    var samples: [Double] = []
    for _ in 0..<50 {
        let start = ContinuousClock.now
        await dispatcher.dispatchTargeted(query, moduleID: .killProcess) { _ in }
        let elapsed = start.duration(to: .now)
        samples.append(
            Double(elapsed.components.seconds) * 1000
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        )
    }

    #expect(p95(samples) < 200)
}

@Test func slowModuleTabTargetedQueryStaysUnderBudget() async {
    let host = ModuleHost(context: moduleContext())
    let service = BrowserTabsService()
    await host.register(BrowserTabsModule(service: service))
    await host.applyEnabledSet([.browserTabs])
    let dispatcher = QueryDispatcher(host: host)
    let query = targetedQuery(raw: "tab github")

    var samples: [Double] = []
    for _ in 0..<10 {
        let start = ContinuousClock.now
        await dispatcher.dispatchTargeted(query, moduleID: .browserTabs) { _ in }
        let elapsed = start.duration(to: .now)
        samples.append(
            Double(elapsed.components.seconds) * 1000
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        )
    }

    #expect(p95(samples) < 950)
}

@Test func slowModuleTabWarmCacheQueryStaysUnderFastBudget() async {
    let host = ModuleHost(context: moduleContext())
    let service = BrowserTabsService()
    await host.register(BrowserTabsModule(service: service))
    await host.applyEnabledSet([.browserTabs])
    await service.refresh()
    let dispatcher = QueryDispatcher(host: host)
    let query = targetedQuery(raw: "tab github")

    var samples: [Double] = []
    for _ in 0..<100 {
        let start = ContinuousClock.now
        await dispatcher.dispatchTargeted(query, moduleID: .browserTabs) { _ in }
        let elapsed = start.duration(to: .now)
        samples.append(
            Double(elapsed.components.seconds) * 1000
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        )
    }

    #expect(p95(samples) < 50)
}
