import Foundation
import LumaCore

/// Launcher UI callbacks installed once at startup via `LauncherEnvironment`.
@MainActor
public struct LauncherCallbacks {
    public var openModuleDetail: (ModuleIdentifier) -> Void
    public var onOpenSettings: () -> Void
    public var onReloadModules: () -> Void
    public var onBackFromDetail: () -> Void
    public var onTranslateContentChanged: (String, String) -> Void
    public var onSecretsLockStateChanged: (Bool) -> Void
    public var onHideLauncher: () -> Void
    public var reloadSecretsDetail: () -> Void
    public var reloadSnippetsDetail: () -> Void
    public var reloadMediaDetail: () -> Void

    public init(
        openModuleDetail: @escaping (ModuleIdentifier) -> Void,
        onOpenSettings: @escaping () -> Void,
        onReloadModules: @escaping () -> Void,
        onBackFromDetail: @escaping () -> Void,
        onTranslateContentChanged: @escaping (String, String) -> Void,
        onSecretsLockStateChanged: @escaping (Bool) -> Void,
        onHideLauncher: @escaping () -> Void,
        reloadSecretsDetail: @escaping () -> Void,
        reloadSnippetsDetail: @escaping () -> Void,
        reloadMediaDetail: @escaping () -> Void
    ) {
        self.openModuleDetail = openModuleDetail
        self.onOpenSettings = onOpenSettings
        self.onReloadModules = onReloadModules
        self.onBackFromDetail = onBackFromDetail
        self.onTranslateContentChanged = onTranslateContentChanged
        self.onSecretsLockStateChanged = onSecretsLockStateChanged
        self.onHideLauncher = onHideLauncher
        self.reloadSecretsDetail = reloadSecretsDetail
        self.reloadSnippetsDetail = reloadSnippetsDetail
        self.reloadMediaDetail = reloadMediaDetail
    }
}

/// Installed from `LauncherEnvironment`; modules read callbacks here instead of a singleton class.
@MainActor
public enum LauncherCallbackRegistry {
    public private(set) static var current: LauncherCallbacks?

    public static func install(_ callbacks: LauncherCallbacks) {
        current = callbacks
    }
}

/// Cross-module mutable launcher state (e.g. pending media editor draft).
@MainActor
public enum LauncherSharedState {
    public static var pendingMediaEditorDraft: MediaEditorDraft?
    public static var pendingWordbookAutoStartReview = false
}
