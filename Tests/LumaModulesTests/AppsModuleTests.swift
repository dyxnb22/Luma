import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private actor CountingRunningApplicationsClient: RunningApplicationsClient {
    private(set) var runningBundleIDsCallCount = 0
    private(set) var startMonitoringCallCount = 0
    let bundleIDs: Set<String>

    init(bundleIDs: Set<String> = ["com.apple.Safari"]) {
        self.bundleIDs = bundleIDs
    }

    func runningBundleIDs() async -> Set<String> {
        runningBundleIDsCallCount += 1
        return bundleIDs
    }

    func startMonitoring() async {
        startMonitoringCallCount += 1
    }

    func stopMonitoring() async {}
}

private func moduleContext(runningApplications: any RunningApplicationsClient) -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        runningApplications: runningApplications
    )
}

@Test func appsModuleHandleUsesCachedRunningApplicationsClient() async {
    let module = AppsModule()
    let runningClient = CountingRunningApplicationsClient()
    let context = moduleContext(runningApplications: runningClient)
    await module.warmup(context)

    let query = Query(raw: "safari", sequence: 1)
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1), platform: QueryPlatformClients(runningApplications: runningClient)))

    #expect(!result.items.isEmpty)
    let countAfterWarmup = await runningClient.runningBundleIDsCallCount
    _ = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1), platform: QueryPlatformClients(runningApplications: runningClient)))
    #expect(await runningClient.runningBundleIDsCallCount == countAfterWarmup)
    await module.teardown()
}

@Test func appsModuleWarmupStartsRunningApplicationsMonitoring() async {
    let module = AppsModule()
    let runningClient = CountingRunningApplicationsClient()
    await module.warmup(moduleContext(runningApplications: runningClient))
    #expect(await runningClient.startMonitoringCallCount == 1)
    await module.teardown()
}

@Test func appsModuleMarksRunningAppsInSubtitle() async {
    let module = AppsModule()
    let runningClient = CountingRunningApplicationsClient(bundleIDs: ["com.apple.Safari"])
    let context = moduleContext(runningApplications: runningClient)
    await module.warmup(context)

    let query = Query(raw: "safari", sequence: 1)
    let platform = QueryPlatformClients(runningApplications: runningClient)
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1), platform: platform))

    let safariRow = result.items.first { $0.id.key == "com.apple.Safari" }
    #expect(safariRow?.subtitle?.contains("Running") == true)
    await module.teardown()
}

@Test func appsMemoryTopDoesNotReturnWarmingOnlyAfterTTLWhenCacheExists() async throws {
    let module = AppsModule()
    let memoryClient = MutableProcessMemoryClient(samples: [
        RunningApplicationMemory(bundleID: "com.apple.Safari", name: "Safari", residentBytes: 50_000_000)
    ])
    let context = ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        processMemory: memoryClient
    )
    await module.warmup(context)

    let queryContext = QueryContext(
        deadline: .now + .seconds(5),
        platform: QueryPlatformClients(processMemory: memoryClient)
    )
    _ = await module.handle(Query(raw: "app top", sequence: 1), context: queryContext)
    try await Task.sleep(for: .milliseconds(2100))
    let expired = await module.handle(Query(raw: "app top", sequence: 2), context: queryContext)
    #expect(expired.items.isEmpty == false)
    #expect(expired.diagnostic?.message != "Memory usage cache warming")
    await module.teardown()
}

private actor MutableProcessMemoryClient: ProcessMemoryClient {
    var samples: [RunningApplicationMemory]

    init(samples: [RunningApplicationMemory]) {
        self.samples = samples
    }

    func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        Array(samples.prefix(limit))
    }
}
