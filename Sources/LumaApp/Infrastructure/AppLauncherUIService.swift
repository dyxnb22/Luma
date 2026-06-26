import AppKit
import LumaCore

@MainActor
final class AppLauncherUIService: LauncherUIClient {
    private let onSecretsLockStateChanged: (Bool) -> Void
    private let onHideLauncher: () -> Void

    init(
        onSecretsLockStateChanged: @escaping (Bool) -> Void,
        onHideLauncher: @escaping () -> Void
    ) {
        self.onSecretsLockStateChanged = onSecretsLockStateChanged
        self.onHideLauncher = onHideLauncher
    }

    func notifySecretsLockStateChanged(_ locked: Bool) async {
        onSecretsLockStateChanged(locked)
    }

    func reloadModuleDetail(_ moduleID: ModuleIdentifier) async {
        switch moduleID {
        case .secrets:
            ModuleDetailReloads.reloadSecretsDetail?()
        case .snippets:
            ModuleDetailReloads.reloadSnippetsDetail?()
        case .media:
            ModuleDetailReloads.reloadMediaDetail?()
        default:
            break
        }
    }

    func hideLauncher() async {
        onHideLauncher()
    }
}
