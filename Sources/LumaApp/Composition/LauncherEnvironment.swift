import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// Injected main-actor dependencies for launcher UI, detail views, and module callbacks.
@MainActor
final class LauncherEnvironment {
    static weak var current: LauncherEnvironment?

    var isLauncherQueryEmpty = true

    let openModuleDetail: (ModuleIdentifier) -> Void
    let openSettings: () -> Void
    let reloadModules: () -> Void
    let onBackFromDetail: () -> Void
    let onTranslateContentChanged: (String, String) -> Void
    let onHideLauncher: () -> Void
    var showStatus: ((String) -> Void)?
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
    let runProjectAction: (ProjectAction, @escaping () -> Void) -> Void

    init(
        openModuleDetail: @escaping (ModuleIdentifier) -> Void,
        openSettings: @escaping () -> Void,
        reloadModules: @escaping () -> Void,
        onBackFromDetail: @escaping () -> Void,
        onTranslateContentChanged: @escaping (String, String) -> Void,
        onHideLauncher: @escaping () -> Void,
        detailReloadRouter: ModuleDetailReloadRouter,
        clipboardModule: ClipboardModule,
        notesModule: NotesModule,
        snippetsModule: SnippetsModule,
        secretsModule: SecretsModule,
        mediaModule: MediaModule,
        todoModule: TodoModule,
        wordbookStore: WordbookStore,
        projectsModule: ProjectsModule,
        quicklinksModule: QuicklinksModule,
        translation: any TranslationClient,
        config: ConfigurationStore,
        runProjectAction: @escaping (ProjectAction, @escaping () -> Void) -> Void
    ) {
        self.openModuleDetail = openModuleDetail
        self.openSettings = openSettings
        self.reloadModules = reloadModules
        self.onBackFromDetail = onBackFromDetail
        self.onTranslateContentChanged = onTranslateContentChanged
        self.onHideLauncher = onHideLauncher
        self.detailReloadRouter = detailReloadRouter
        self.clipboardModule = clipboardModule
        self.notesModule = notesModule
        self.snippetsModule = snippetsModule
        self.secretsModule = secretsModule
        self.mediaModule = mediaModule
        self.todoModule = todoModule
        self.wordbookStore = wordbookStore
        self.projectsModule = projectsModule
        self.quicklinksModule = quicklinksModule
        self.translation = translation
        self.config = config
        self.runProjectAction = runProjectAction
    }

    func install() {
        LauncherEnvironment.current = self
    }

    func makeDetailView(for id: ModuleIdentifier) -> (any ModuleDetailView)? {
        switch id {
        case .translate:
            return TranslateDetailView(translation: translation, config: config) { [weak self] source, output in
                self?.onTranslateContentChanged(source, output)
            }
        case .clipboard:
            return ClipboardDetailView(module: clipboardModule, onOpenSettings: { [weak self] in
                self?.openSettings()
            }, onHideLauncher: { [weak self] in
                self?.onHideLauncher()
            })
        case .notes:
            return NotesDetailView(module: notesModule)
        case .snippets:
            return SnippetsDetailView(module: snippetsModule, detailReloadRouter: detailReloadRouter)
        case .secrets:
            return SecretsDetailView(module: secretsModule, detailReloadRouter: detailReloadRouter)
        case .media:
            return MediaDetailView(module: mediaModule, detailReloadRouter: detailReloadRouter)
        case .todo:
            return TodoDetailView(module: todoModule)
        case .wordbook:
            return WordbookDetailView(store: wordbookStore)
        case .projects:
            if LauncherSharedState.pendingProjectsManage {
                LauncherSharedState.pendingProjectsManage = false
                return ProjectsDetailView(module: projectsModule, onRunProjectAction: runProjectAction)
            }
            return CurrentProjectDetailView(onRunProjectAction: runProjectAction)
        case .quicklinks:
            return QuicklinksDetailView(module: quicklinksModule)
        default:
            return nil
        }
    }
}
