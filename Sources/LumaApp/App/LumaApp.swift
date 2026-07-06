@preconcurrency import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@main
@MainActor
final class LumaApplication: NSObject, NSApplicationDelegate {
    private var coordinator: AppCoordinator?

    nonisolated static func main() {
        let app = NSApplication.shared
        let delegate = LumaApplication()
        app.delegate = delegate
        app.setActivationPolicy(.accessory)
        app.run()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        coordinator = AppCoordinator()
        coordinator?.start()
    }
}
