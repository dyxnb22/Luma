import Foundation

/// Detail views register reload handlers here; `LauncherEnvironment` forwards module callbacks.
@MainActor
enum ModuleDetailReloads {
    static var reloadSecretsDetail: (() -> Void)?
    static var reloadSnippetsDetail: (() -> Void)?
    static var reloadMediaDetail: (() -> Void)?
}
