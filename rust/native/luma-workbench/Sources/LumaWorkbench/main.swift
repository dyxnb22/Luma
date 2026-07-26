import AppKit

// Accessory app: menu bar and window when active, no permanent Dock icon.
let application = NSApplication.shared
application.setActivationPolicy(.accessory)

// NSApplication holds its delegate weakly; this top-level binding owns it.
let appDelegate = AppDelegate()
application.delegate = appDelegate

application.run()
