import AppKit
import LumaCore

@MainActor
final class AppLauncherUIService: LauncherUIClient {
    private let detailReloadRouter: ModuleDetailReloadRouter
    private let onSecretsLockStateChanged: (Bool) -> Void
    private let onHideLauncher: () -> Void

    init(
        detailReloadRouter: ModuleDetailReloadRouter,
        onSecretsLockStateChanged: @escaping (Bool) -> Void,
        onHideLauncher: @escaping () -> Void
    ) {
        self.detailReloadRouter = detailReloadRouter
        self.onSecretsLockStateChanged = onSecretsLockStateChanged
        self.onHideLauncher = onHideLauncher
    }

    func notifySecretsLockStateChanged(_ locked: Bool) async {
        onSecretsLockStateChanged(locked)
    }

    func reloadModuleDetail(_ moduleID: ModuleIdentifier) async {
        detailReloadRouter.reload(moduleID)
    }

    func hideLauncher() async {
        onHideLauncher()
    }
}
