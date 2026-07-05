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

private struct StubProcessMemorySampler: ProcessMemoryClient {
    func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        _ = limit
        return [
            RunningApplicationMemory(bundleID: "com.apple.Safari", name: "Safari", residentBytes: 512 * 1024 * 1024)
        ]
    }
}

private func moduleContext(processMemory: any ProcessMemoryClient) -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        processMemory: processMemory
    )
}

@Test func appsModuleTopTargetedQueryStaysUnderBudget() async {
    let host = ModuleHost(context: moduleContext(processMemory: StubProcessMemorySampler()))
    await host.register(AppsModule())
    let dispatcher = QueryDispatcher(host: host)
    let parsed = ParsedCommand(trigger: "app", payload: "top", module: .apps)
    let query = Query(raw: "app top", sequence: 1, command: parsed)

    var samples: [Double] = []
    for _ in 0..<100 {
        let start = ContinuousClock.now
        await dispatcher.dispatchTargeted(query, moduleID: .apps) { _ in }
        let elapsed = start.duration(to: .now)
        samples.append(
            Double(elapsed.components.seconds) * 1000
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        )
    }

    #expect(p95(samples) < 40)
}
