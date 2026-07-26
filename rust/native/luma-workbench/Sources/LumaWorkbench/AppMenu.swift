import AppKit
import LumaWorkbenchCore

/// The smallest menu that still gives macOS the standard shortcuts.
///
/// An accessory application has no visible menu bar, so this exists purely to bind key
/// equivalents: Cmd+C, Cmd+V and Cmd+A reach the terminal view, Cmd+W hides, Cmd+Q quits. No
/// module surfaces and no preferences window.
enum AppMenu {
    static func make() -> NSMenu {
        let mainMenu = NSMenu()
        mainMenu.addItem(applicationMenuItem())
        mainMenu.addItem(editMenuItem())
        return mainMenu
    }

    private static func applicationMenuItem() -> NSMenuItem {
        let name = HostIdentity.applicationName
        let menu = NSMenu(title: name)

        // Close routes through the window delegate, which hides instead of terminating.
        menu.addItem(
            withTitle: "Close Window",
            action: #selector(NSWindow.performClose(_:)),
            keyEquivalent: "w"
        )
        menu.addItem(.separator())

        menu.addItem(
            withTitle: "Quit \(name)",
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q"
        )

        let item = NSMenuItem()
        item.submenu = menu
        return item
    }

    private static func editMenuItem() -> NSMenuItem {
        let menu = NSMenu(title: "Edit")
        menu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        menu.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        menu.addItem(
            withTitle: "Select All",
            action: #selector(NSText.selectAll(_:)),
            keyEquivalent: "a"
        )

        let item = NSMenuItem()
        item.submenu = menu
        return item
    }
}
