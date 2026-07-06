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
    let openTranslationSettings: () -> Void
    let reloadModules: () -> Void
    let onBackFromDetail: () -> Void
    let onTranslateContentChanged: (String, String) -> Void
    let onHideLauncher: () -> Void
    let showStatus: (String) -> Void
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
    let accessibility: any AccessibilityClient
    let runProjectAction: (ProjectAction, @escaping () -> Void) -> Void
    let runWorkbenchCapture: (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void
    let runWorkspaceRow: (CurrentProjectWorkspaceRowAction) -> Void
    let warmModuleForDetail: (ModuleIdentifier) async -> Void
    let reserveDetailModule: (ModuleIdentifier?) async -> Void

    private let detailRegistry: ModuleDetailRegistry

    init(
        openModuleDetail: @escaping (ModuleIdentifier) -> Void,
        openSettings: @escaping () -> Void,
        openTranslationSettings: (() -> Void)? = nil,
        reloadModules: @escaping () -> Void,
        onBackFromDetail: @escaping () -> Void,
        onTranslateContentChanged: @escaping (String, String) -> Void,
        onHideLauncher: @escaping () -> Void,
        showStatus: @escaping (String) -> Void,
        detailReloadRouter: ModuleDetailReloadRouter,
        detailRegistry: ModuleDetailRegistry = .makeDefault(),
        warmModuleForDetail: @escaping (ModuleIdentifier) async -> Void = { _ in },
        reserveDetailModule: @escaping (ModuleIdentifier?) async -> Void = { _ in },
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
        accessibility: any AccessibilityClient,
        runProjectAction: @escaping (ProjectAction, @escaping () -> Void) -> Void,
        runWorkbenchCapture: @escaping (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void = { _, _ in },
        runWorkspaceRow: @escaping (CurrentProjectWorkspaceRowAction) -> Void = { _ in }
    ) {
        self.openModuleDetail = openModuleDetail
        self.openSettings = openSettings
        self.openTranslationSettings = openTranslationSettings ?? openSettings
        self.reloadModules = reloadModules
        self.onBackFromDetail = onBackFromDetail
        self.onTranslateContentChanged = onTranslateContentChanged
        self.onHideLauncher = onHideLauncher
        self.showStatus = showStatus
        self.detailReloadRouter = detailReloadRouter
        self.detailRegistry = detailRegistry
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
        self.accessibility = accessibility
        self.runProjectAction = runProjectAction
        self.runWorkbenchCapture = runWorkbenchCapture
        self.runWorkspaceRow = runWorkspaceRow
        self.warmModuleForDetail = warmModuleForDetail
        self.reserveDetailModule = reserveDetailModule
    }

    func install() {
        LauncherEnvironment.current = self
    }

    func makeDetailView(for id: ModuleIdentifier) -> (any ModuleDetailView)? {
        detailRegistry.makeDetailView(for: id, context: uiContext)
    }

    func activateDetail(_ detail: any ModuleDetailView, for moduleID: ModuleIdentifier) async {
        await detailRegistry.activateDetailView(detail, moduleID: moduleID)
    }

    func canMakeDetailView(for id: ModuleIdentifier) -> Bool {
        detailRegistry.hasFactory(for: id)
    }

    private var uiContext: ModuleUIContext {
        ModuleUIContext(
            detailReloadRouter: detailReloadRouter,
            clipboardModule: clipboardModule,
            notesModule: notesModule,
            snippetsModule: snippetsModule,
            secretsModule: secretsModule,
            mediaModule: mediaModule,
            todoModule: todoModule,
            wordbookStore: wordbookStore,
            projectsModule: projectsModule,
            quicklinksModule: quicklinksModule,
            translation: translation,
            config: config,
            onOpenSettings: openSettings,
            onOpenTranslationSettings: openTranslationSettings,
            onHideLauncher: onHideLauncher,
            accessibility: accessibility,
            onTranslateContentChanged: onTranslateContentChanged,
            runProjectAction: runProjectAction,
            runWorkbenchCapture: runWorkbenchCapture,
            runWorkspaceRow: runWorkspaceRow
        )
    }
}
