import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

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
    let onHideLauncher: () -> Void
    let onTranslateContentChanged: (String, String) -> Void
    let runProjectAction: (ProjectAction, @escaping () -> Void) -> Void
    let runWorkbenchCapture: (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void
    let runWorkspaceRow: (CurrentProjectWorkspaceRowAction) -> Void
}

@MainActor
final class ModuleDetailRegistry {
    typealias Factory = (ModuleUIContext) -> (any ModuleDetailView)?

    private var factories: [ModuleIdentifier: Factory] = [:]

    func register(_ id: ModuleIdentifier, factory: @escaping Factory) {
        factories[id] = factory
    }

    func makeDetailView(for id: ModuleIdentifier, context: ModuleUIContext) -> (any ModuleDetailView)? {
        factories[id]?(context)
    }

    static func makeDefault() -> ModuleDetailRegistry {
        let registry = ModuleDetailRegistry()
        registry.register(.translate) { ctx in
            TranslateDetailView(translation: ctx.translation, config: ctx.config) { source, output in
                ctx.onTranslateContentChanged(source, output)
            }
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
