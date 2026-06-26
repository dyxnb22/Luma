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
    let onSecretsLockStateChanged: (Bool) -> Void
    let onHideLauncher: () -> Void
    let reloadSecretsDetail: () -> Void
    let reloadSnippetsDetail: () -> Void
    let reloadMediaDetail: () -> Void

    let clipboardModule: ClipboardModule
    let notesModule: NotesModule
    let snippetsModule: SnippetsModule
    let secretsModule: SecretsModule
    let mediaModule: MediaModule
    let todoModule: TodoModule
    let wordbookStore: WordbookStore
    let translation: any TranslationClient
    let config: ConfigurationStore

    init(
        openModuleDetail: @escaping (ModuleIdentifier) -> Void,
        openSettings: @escaping () -> Void,
        reloadModules: @escaping () -> Void,
        onBackFromDetail: @escaping () -> Void,
        onTranslateContentChanged: @escaping (String, String) -> Void,
        onSecretsLockStateChanged: @escaping (Bool) -> Void,
        onHideLauncher: @escaping () -> Void,
        reloadSecretsDetail: @escaping () -> Void,
        reloadSnippetsDetail: @escaping () -> Void,
        reloadMediaDetail: @escaping () -> Void,
        clipboardModule: ClipboardModule,
        notesModule: NotesModule,
        snippetsModule: SnippetsModule,
        secretsModule: SecretsModule,
        mediaModule: MediaModule,
        todoModule: TodoModule,
        wordbookStore: WordbookStore,
        translation: any TranslationClient,
        config: ConfigurationStore
    ) {
        self.openModuleDetail = openModuleDetail
        self.openSettings = openSettings
        self.reloadModules = reloadModules
        self.onBackFromDetail = onBackFromDetail
        self.onTranslateContentChanged = onTranslateContentChanged
        self.onSecretsLockStateChanged = onSecretsLockStateChanged
        self.onHideLauncher = onHideLauncher
        self.reloadSecretsDetail = reloadSecretsDetail
        self.reloadSnippetsDetail = reloadSnippetsDetail
        self.reloadMediaDetail = reloadMediaDetail
        self.clipboardModule = clipboardModule
        self.notesModule = notesModule
        self.snippetsModule = snippetsModule
        self.secretsModule = secretsModule
        self.mediaModule = mediaModule
        self.todoModule = todoModule
        self.wordbookStore = wordbookStore
        self.translation = translation
        self.config = config
    }

    func install() {
        LauncherEnvironment.current = self
        LauncherCallbackRegistry.install(LauncherCallbacks(
            openModuleDetail: openModuleDetail,
            onOpenSettings: openSettings,
            onReloadModules: reloadModules,
            onBackFromDetail: onBackFromDetail,
            onTranslateContentChanged: onTranslateContentChanged,
            onSecretsLockStateChanged: onSecretsLockStateChanged,
            onHideLauncher: onHideLauncher,
            reloadSecretsDetail: reloadSecretsDetail,
            reloadSnippetsDetail: reloadSnippetsDetail,
            reloadMediaDetail: reloadMediaDetail
        ))
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
            })
        case .notes:
            return NotesDetailView(module: notesModule)
        case .snippets:
            return SnippetsDetailView(module: snippetsModule)
        case .secrets:
            return SecretsDetailView(module: secretsModule)
        case .media:
            return MediaDetailView(module: mediaModule)
        case .todo:
            return TodoDetailView(module: todoModule)
        case .wordbook:
            return WordbookDetailView(store: wordbookStore)
        default:
            return nil
        }
    }
}
