import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules

/// Injected main-actor dependencies for launcher UI and module callbacks.
@MainActor
struct LauncherEnvironment {
    let openModuleDetail: (ModuleIdentifier) -> Void
    let openSettings: () -> Void
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

    func installCallbacks() {
        LauncherCallbackRegistry.install(LauncherCallbacks(
            openModuleDetail: openModuleDetail,
            onOpenSettings: openSettings,
            onBackFromDetail: onBackFromDetail,
            onTranslateContentChanged: onTranslateContentChanged,
            onSecretsLockStateChanged: onSecretsLockStateChanged,
            onHideLauncher: onHideLauncher,
            reloadSecretsDetail: reloadSecretsDetail,
            reloadSnippetsDetail: reloadSnippetsDetail,
            reloadMediaDetail: reloadMediaDetail
        ))
    }

    func applyToModuleDetailRegistry() {
        ModuleDetailRegistry.clipboardModule = clipboardModule
        ModuleDetailRegistry.notesModule = notesModule
        ModuleDetailRegistry.snippetsModule = snippetsModule
        ModuleDetailRegistry.secretsModule = secretsModule
        ModuleDetailRegistry.mediaModule = mediaModule
        ModuleDetailRegistry.todoModule = todoModule
        ModuleDetailRegistry.wordbookStore = wordbookStore
        ModuleDetailRegistry.translation = translation
        ModuleDetailRegistry.config = config
    }
}
