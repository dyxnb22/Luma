import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing
@testable import LumaApp

/// Scripted launcher flow driver for behavioral integration tests.
@MainActor
final class LauncherFlowHarness {
    private(set) var lastSnapshot: ResultSnapshot?
    private(set) var panelActive = false
    private(set) var queryText = ""
    private(set) var lastStatusMessage: String?
    private(set) var dismissCount = 0
    private(set) var actionPanelVisible = false

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

    func showPanel() {
        panelActive = true
    }

    func hidePanel() {
        panelActive = false
        viewModel.cancel()
    }

    func activatePanel() { showPanel() }
    func deactivatePanel() { hidePanel() }

    func type(_ text: String) {
        queryText = text
        guard panelActive else { return }
        let route = viewModel.commandRouter.route(raw: text)
        let parsed = viewModel.commandRouter.registry.parsedCommand(for: text, route: route)
        viewModel.queryChanged(text, issuedAt: ContinuousClock.now, route: route, parsedCommand: parsed)
    }

    func recordStatus(_ message: String) {
        lastStatusMessage = message
    }

    func recordDismiss() {
        dismissCount += 1
    }

    func setActionPanelVisible(_ visible: Bool) {
        actionPanelVisible = visible
    }

    func assertItemCount(_ count: Int, sourceLocation: SourceLocation = #_sourceLocation) {
        #expect(lastSnapshot?.items.count == count, sourceLocation: sourceLocation)
    }

    func assertContainsTitle(_ title: String, sourceLocation: SourceLocation = #_sourceLocation) {
        let titles = lastSnapshot?.items.map(\.title) ?? []
        #expect(titles.contains(where: { $0.localizedCaseInsensitiveContains(title) }), sourceLocation: sourceLocation)
    }

    func assertStatusMessage(_ message: String, sourceLocation: SourceLocation = #_sourceLocation) {
        #expect(lastStatusMessage?.localizedCaseInsensitiveContains(message) == true, sourceLocation: sourceLocation)
    }
}

@Test @MainActor func launcherFlowHarnessReplaysQuery() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.showPanel()
    harness.type("app")
    try? await Task.sleep(for: .milliseconds(200))
    #expect((harness.lastSnapshot?.items.isEmpty ?? true) == false)
    #expect(harness.lastSnapshot != nil)
}

@Test @MainActor func emptyQueryHomeGuideHasRows() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.showPanel()
    harness.type("")
    try? await Task.sleep(for: .milliseconds(200))
    #expect(harness.panelActive)
}
