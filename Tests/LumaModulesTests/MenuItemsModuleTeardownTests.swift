import Foundation
import LumaCore
import LumaModules
import Testing
@testable import LumaServices

@Test func menuItemsModuleTeardownStopsServiceObserver() async {
    let service = MenuBarTreeService()
    let module = MenuItemsModule(service: service)
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
    #expect(await service.hasObserverInstalled == true)

    await module.teardown()
    #expect(await service.hasObserverInstalled == false)
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
