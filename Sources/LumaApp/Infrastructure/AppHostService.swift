import AppKit
import Foundation
import LumaCore

@MainActor
final class AppHostService: HostClient {
    private let onOpenSettings: () -> Void
    private let onReloadModules: () -> Void

    init(
        onOpenSettings: @escaping () -> Void,
        onReloadModules: @escaping () -> Void
    ) {
        self.onOpenSettings = onOpenSettings
        self.onReloadModules = onReloadModules
    }

    func openSettings() async {
        onOpenSettings()
    }

    func reloadModules() async {
        onReloadModules()
    }

    func quitHost() async {
        NSApp.terminate(nil)
    }
}
