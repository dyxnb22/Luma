import AppKit
import LumaWorkbenchCore

let sessionLock: SingleInstanceLock
do {
    let lockURL = HostIdentity.singleInstanceLockURL(
        temporaryDirectory: FileManager.default.temporaryDirectory
    )
    guard let acquiredLock = try SingleInstanceLock.acquire(at: lockURL) else {
        DistributedNotificationCenter.default().postNotificationName(
            Notification.Name(HostIdentity.showRunningInstanceNotification),
            object: nil,
            deliverImmediately: true
        )
        exit(EXIT_SUCCESS)
    }
    sessionLock = acquiredLock
} catch {
    fputs("\(HostIdentity.applicationName) cannot acquire its session lock: \(error)\n", stderr)
    exit(EXIT_FAILURE)
}

// Accessory app: menu bar and window when active, no permanent Dock icon.
let application = NSApplication.shared
application.setActivationPolicy(.accessory)

// NSApplication holds its delegate weakly; this top-level binding owns it.
let appDelegate = AppDelegate()
application.delegate = appDelegate

withExtendedLifetime(sessionLock) {
    application.run()
}
