import Foundation

public protocol LauncherUIClient: Sendable {
    func notifySecretsLockStateChanged(_ locked: Bool) async
    func reloadModuleDetail(_ moduleID: ModuleIdentifier) async
    func hideLauncher() async
}

public struct NoopLauncherUIClient: LauncherUIClient {
    public init() {}

    public func notifySecretsLockStateChanged(_ locked: Bool) async {}
    public func reloadModuleDetail(_ moduleID: ModuleIdentifier) async {}
    public func hideLauncher() async {}
}
