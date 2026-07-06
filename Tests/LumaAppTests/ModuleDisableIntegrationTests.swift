import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import Testing
@testable import LumaApp

@MainActor
private final class MockModuleDetail: NSObject, ModuleDetailView {
    let detailView = NSView()
    let moduleTitle = "Mock"
    let usesSharedTopBar = false
    var detailContentGeneration: UInt64 = 1
    func refreshDetailContentGeneration() async {}
    func activate() {}
    func activate(generation: UInt64) {}
    func deactivate() {}
    func handleKeyDown(_ event: NSEvent) -> Bool { false }
    func prepareForLauncherHide() async {}
}

@Test @MainActor func moduleDetailRegistryEvictsRemovedModules() {
    let registry = ModuleDetailRegistry()
    var makeCount = 0
    registry.register(.clipboard) { _ in
        makeCount += 1
        return MockModuleDetail()
    }
    let context = ModuleUIContext(
        detailReloadRouter: ModuleDetailReloadRouter(),
        clipboardModule: ClipboardModule(pasteboard: NoopPasteboardClient(), accessibility: NoopAccessibilityClient()),
        notesModule: NotesModule(),
        snippetsModule: SnippetsModule(),
        secretsModule: SecretsModule(),
        mediaModule: MediaModule(),
        todoModule: TodoModule(),
        wordbookStore: WordbookStore(),
        projectsModule: ProjectsModule(),
        quicklinksModule: QuicklinksModule(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        onOpenSettings: {},
        onOpenTranslationSettings: {},
        onHideLauncher: {},
        accessibility: NoopAccessibilityClient(),
        onTranslateContentChanged: { _, _ in },
        runProjectAction: { _, _ in },
        runWorkbenchCapture: { _, _ in },
        runWorkspaceRow: { _ in }
    )
    _ = registry.makeDetailView(for: .clipboard, context: context)
    #expect(makeCount == 1)
    registry.evict([.clipboard])
    _ = registry.makeDetailView(for: .clipboard, context: context)
    #expect(makeCount == 2)
}

@Test func queryDispatcherCancelRevalidationIsReachable() async {
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
    await dispatcher.invalidateSnapshotCache()
}

import LumaModules
import LumaServices
