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
    let reloadSnippetsDetail: () -> Void

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
        onHideLauncher: @escaping () -> Void,
        reloadSnippetsDetail: @escaping () -> Void,
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
        self.onHideLauncher = onHideLauncher
        self.reloadSnippetsDetail = reloadSnippetsDetail
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
