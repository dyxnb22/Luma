import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// Shared dependencies for constructing in-panel module detail views.
@MainActor
struct ModuleUIContext {
    let detailReloadRouter: ModuleDetailReloadRouter
    let clipboardModule: ClipboardModule
    let notesModule: NotesModule
    let snippetsModule: SnippetsModule
    let secretsModule: SecretsModule
    let mediaModule: MediaModule
    let todoModule: TodoModule
    let wordbookStore: WordbookStore
    let projectsModule: ProjectsModule
    let quicklinksModule: QuicklinksModule
    let translation: any TranslationClient
    let config: ConfigurationStore
    let onOpenSettings: () -> Void
    let onOpenTranslationSettings: () -> Void
    let onHideLauncher: () -> Void
    let accessibility: any AccessibilityClient
    let onTranslateContentChanged: (String, String) -> Void
    let runProjectAction: (ProjectAction, @escaping () -> Void) -> Void
    let runWorkbenchCapture: (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void
    let runWorkspaceRow: (CurrentProjectWorkspaceRowAction) -> Void
}

@MainActor
final class ModuleDetailRegistry {
    typealias Factory = (ModuleUIContext) -> (any ModuleDetailView)?

    private var factories: [ModuleIdentifier: Factory] = [:]
    private var detailPool: [ModuleIdentifier: any ModuleDetailView] = [:]
    private var lastActivatedGeneration: [ModuleIdentifier: UInt64] = [:]

    func register(_ id: ModuleIdentifier, factory: @escaping Factory) {
        factories[id] = factory
    }

    func hasFactory(for id: ModuleIdentifier) -> Bool {
        factories[id] != nil
    }

    func makeDetailView(for id: ModuleIdentifier, context: ModuleUIContext) -> (any ModuleDetailView)? {
        if let cached = detailPool[id] {
            return cached
        }
        guard let detail = factories[id]?(context) else { return nil }
        detailPool[id] = detail
        LauncherPerfCounters.increment(.detailViewMade)
        return detail
    }

    func activateDetailView(_ detail: any ModuleDetailView, moduleID: ModuleIdentifier) async {
        await detail.refreshDetailContentGeneration()
        let generation = detail.detailContentGeneration
        // Skip only when generation > 0: default protocol views (0) always activate;
        // Clipboard/Todo/Notes at revision 0 also pay full activation until first mutation.
        if lastActivatedGeneration[moduleID] == generation, generation > 0 {
            return
        }
        lastActivatedGeneration[moduleID] = generation
        detail.activate(generation: generation)
    }

    func evict(_ ids: Set<ModuleIdentifier>) {
        for id in ids {
            detailPool.removeValue(forKey: id)
            lastActivatedGeneration.removeValue(forKey: id)
        }
    }

    static func makeDefault() -> ModuleDetailRegistry {
        let registry = ModuleDetailRegistry()
        registry.register(.translate) { ctx in
            TranslateDetailView(
                translation: ctx.translation,
                config: ctx.config,
                accessibility: ctx.accessibility,
                onContentChanged: { source, output in
                    ctx.onTranslateContentChanged(source, output)
                },
                onOpenTranslationSettings: ctx.onOpenTranslationSettings,
                onHideLauncher: ctx.onHideLauncher
            )
        }
        registry.register(.clipboard) { ctx in
            ClipboardDetailView(
                module: ctx.clipboardModule,
                onOpenSettings: ctx.onOpenSettings,
                onHideLauncher: ctx.onHideLauncher
            )
        }
        registry.register(.notes) { ctx in
            NotesDetailView(module: ctx.notesModule)
        }
        registry.register(.snippets) { ctx in
            SnippetsDetailView(module: ctx.snippetsModule, detailReloadRouter: ctx.detailReloadRouter)
        }
        registry.register(.secrets) { ctx in
            SecretsDetailView(module: ctx.secretsModule, detailReloadRouter: ctx.detailReloadRouter)
        }
        registry.register(.media) { ctx in
            MediaDetailView(module: ctx.mediaModule, detailReloadRouter: ctx.detailReloadRouter)
        }
        registry.register(.todo) { ctx in
            TodoDetailView(module: ctx.todoModule)
        }
        registry.register(.wordbook) { ctx in
            WordbookDetailView(store: ctx.wordbookStore)
        }
        registry.register(.projects) { ctx in
            if LauncherSharedState.pendingProjectsManage {
                LauncherSharedState.pendingProjectsManage = false
                return ProjectsDetailView(module: ctx.projectsModule, onRunProjectAction: ctx.runProjectAction)
            }
            return CurrentProjectDetailView(
                config: ctx.config,
                onRunProjectAction: ctx.runProjectAction,
                onRunWorkbenchCapture: ctx.runWorkbenchCapture,
                onRunWorkspaceRow: ctx.runWorkspaceRow
            )
        }
        registry.register(.quicklinks) { ctx in
            QuicklinksDetailView(module: ctx.quicklinksModule)
        }
        return registry
    }
}
