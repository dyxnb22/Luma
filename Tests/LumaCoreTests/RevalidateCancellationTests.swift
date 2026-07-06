import Foundation
import LumaCore
import LumaInfrastructure
import LumaServices
import Testing

@Test func queryDispatcherCancelRevalidationIsIdempotent() async {
    let host = ModuleHost(context: ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))
    let dispatcher = QueryDispatcher(host: host)
    await dispatcher.cancelRevalidation()
    await dispatcher.cancelRevalidation()
}
