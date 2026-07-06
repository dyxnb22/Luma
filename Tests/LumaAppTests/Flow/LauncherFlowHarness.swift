import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing
@testable import LumaApp

/// Scripted launcher flow driver for behavioral integration tests.
/// Currently validates dispatcher → view model → snapshot only; does not exercise apply → UI.
@MainActor
final class LauncherFlowHarness {
    private(set) var lastSnapshot: ResultSnapshot?
    private(set) var panelActive = false
    private(set) var queryText = ""

    let viewModel: LauncherViewModel
    private let dispatcher: QueryDispatcher

    init(dispatcher: QueryDispatcher) {
        self.dispatcher = dispatcher
        self.viewModel = LauncherViewModel(dispatcher: dispatcher)
        viewModel.onSnapshot = { [weak self] snapshot in
            self?.lastSnapshot = snapshot
        }
    }

    static func makeWithBuiltInModules() async -> LauncherFlowHarness {
        let context = ModuleContext(
            logger: LumaLogger(),
            metrics: LumaMetrics(),
            database: ApplicationSupportPaths(),
            pasteboard: NoopPasteboardClient(),
            accessibility: NoopAccessibilityClient(),
            fileSystem: NoopFileSystemClient(),
            translation: NoopTranslationClient(),
            config: ConfigurationStore()
        )
        let host = ModuleHost(context: context)
        for module in BuiltInModules.makeAll() {
            await host.register(module)
        }
        await host.warmupAll()
        let dispatcher = QueryDispatcher(host: host, usage: InMemoryUsageTracker())
        return LauncherFlowHarness(dispatcher: dispatcher)
    }

    func activatePanel() {
        panelActive = true
    }

    func deactivatePanel() {
        panelActive = false
        viewModel.cancel()
    }

    func type(_ text: String) {
        queryText = text
        guard panelActive else { return }
        let route = viewModel.commandRouter.route(raw: text)
        let parsed = viewModel.commandRouter.registry.parsedCommand(for: text, route: route)
        viewModel.queryChanged(text, issuedAt: ContinuousClock.now, route: route, parsedCommand: parsed)
    }

    func assertItemCount(_ count: Int, sourceLocation: SourceLocation = #_sourceLocation) {
        #expect(lastSnapshot?.items.count == count, sourceLocation: sourceLocation)
    }

    func assertContainsTitle(_ title: String, sourceLocation: SourceLocation = #_sourceLocation) {
        let titles = lastSnapshot?.items.map(\.title) ?? []
        #expect(titles.contains(where: { $0.localizedCaseInsensitiveContains(title) }), sourceLocation: sourceLocation)
    }
}

@Test @MainActor func launcherFlowHarnessReplaysQuery() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.activatePanel()
    harness.type("app")
    try? await Task.sleep(for: .milliseconds(200))
    #expect((harness.lastSnapshot?.items.isEmpty ?? true) == false)
    #expect(harness.lastSnapshot != nil)
}
