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
    let runningClient = CountingRunningApplicationsClient()
    let module = AppsModule()
    let context = moduleContext(runningApplications: runningClient)
    await module.warmup(context)

    let query = Query(raw: "safari", sequence: 1)
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1), platform: QueryPlatformClients(runningApplications: runningClient)))

    #expect(!result.items.isEmpty)
    let countAfterWarmup = await runningClient.runningBundleIDsCallCount
    _ = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1), platform: QueryPlatformClients(runningApplications: runningClient)))
    #expect(await runningClient.runningBundleIDsCallCount == countAfterWarmup)
}

@Test func appsModuleWarmupStartsRunningApplicationsMonitoring() async {
    let runningClient = CountingRunningApplicationsClient()
    let module = AppsModule()
    await module.warmup(moduleContext(runningApplications: runningClient))
    #expect(await runningClient.startMonitoringCallCount == 1)
}

@Test func appsModuleMarksRunningAppsInSubtitle() async {
    let runningClient = CountingRunningApplicationsClient(bundleIDs: ["com.apple.Safari"])
    let module = AppsModule()
    let context = moduleContext(runningApplications: runningClient)
    await module.warmup(context)

    let query = Query(raw: "safari", sequence: 1)
    let platform = QueryPlatformClients(runningApplications: runningClient)
    let result = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1), platform: platform))

    let safariRow = result.items.first { $0.id.key == "com.apple.Safari" }
    #expect(safariRow?.subtitle?.contains("Running") == true)
}
