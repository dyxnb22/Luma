import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

@Test(.tags(.integration), .enabled(if: IntegrationTestSettings.enabled))
func appScannerIncludesSafariOnThisMachine() {
    let safariURL = URL(fileURLWithPath: "/Applications/Safari.app")
    #expect(FileManager.default.fileExists(atPath: safariURL.path))
    #expect(Bundle(url: safariURL) != nil)

    let scanned = AppScanner.scan()
    let safari = scanned.first { $0.bundleID == "com.apple.Safari" }
    #expect(scanned.count > 0)
    #expect(safari != nil)
    let index = AppIndex(apps: scanned)
    #expect(index.search("safari").first?.bundleID == "com.apple.Safari")
}

@Test(.tags(.integration), .enabled(if: IntegrationTestSettings.enabled))
func appsModuleFindsSafariAfterWarmup() async {
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(200)))
    let module = AppsModule()
    let moduleContext = ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(),
        config: ConfigurationStore()
    )
    await module.warmup(moduleContext)
    let result = await module.handle(Query(raw: "safari", sequence: 1), context: context)
    #expect(!result.items.isEmpty)
    #expect(result.items.contains { $0.title.localizedCaseInsensitiveContains("Safari") })
}

@Test(.tags(.integration), .enabled(if: IntegrationTestSettings.enabled))
func globalSearchDispatchReturnsSafariApp() async {
    let context = ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
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
    await host.configureGlobalSearchModuleIDs(ModuleRegistry.globalSearchModuleIDs)
    await host.warmupAll()

    let dispatcher = QueryDispatcher(host: host)
    final class Box: @unchecked Sendable {
        var snapshot = ResultSnapshot(querySequence: 0, items: [])
    }
    let box = Box()
    await dispatcher.dispatch(Query(raw: "safari", sequence: 1)) { snapshot in
        box.snapshot = snapshot
    }

    #expect(box.snapshot.items.contains { $0.title.localizedCaseInsensitiveContains("Safari") })
}
