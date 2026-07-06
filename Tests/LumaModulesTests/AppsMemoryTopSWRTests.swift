import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private actor MutableProcessMemoryClient: ProcessMemoryClient {
    var samples: [RunningApplicationMemory]

    init(samples: [RunningApplicationMemory]) {
        self.samples = samples
    }

    func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        Array(samples.prefix(limit))
    }
}

private func appsModuleContext(
    runningApplications: any RunningApplicationsClient = NoopRunningApplicationsClient(),
    processMemory: any ProcessMemoryClient
) -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        processMemory: processMemory,
        runningApplications: runningApplications
    )
}

@Test func appsMemoryTopReturnsStaleAfterTTLExpires() async throws {
    let module = AppsModule()
    let memoryClient = MutableProcessMemoryClient(samples: [
        RunningApplicationMemory(bundleID: "com.apple.Safari", name: "Safari", residentBytes: 100_000_000)
    ])
    let context = appsModuleContext(processMemory: memoryClient)
    await module.warmup(context)

    let platform = QueryPlatformClients(processMemory: memoryClient)
    let queryContext = QueryContext(deadline: .now + .seconds(5), platform: platform)
    let fresh = await module.handle(Query(raw: "app top", sequence: 1), context: queryContext)
    #expect(fresh.items.contains(where: { $0.title == "Safari" }))

    try await Task.sleep(for: .milliseconds(2100))
    let stale = await module.handle(Query(raw: "app top", sequence: 2), context: queryContext)
    #expect(stale.items.contains(where: { $0.title == "Safari" }))
    #expect(stale.diagnostic == nil)
    await module.teardown()
}
