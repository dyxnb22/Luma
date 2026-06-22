import AppKit

@MainActor
final class MenuBarController {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let menu = NSMenu()
    private var hotkeyWarningItem: NSMenuItem?
    private let onShow: @MainActor () -> Void
    private let onSettings: @MainActor () -> Void

    init(onShow: @escaping @MainActor () -> Void, onSettings: @escaping @MainActor () -> Void) {
        self.onShow = onShow
        self.onSettings = onSettings
        configure()
        markHotkeyOK()
    }

    func markHotkeyOK() {
        statusItem.button?.title = ""
        let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .semibold)
        statusItem.button?.image = NSImage(systemSymbolName: "command", accessibilityDescription: "Luma")?
            .withSymbolConfiguration(config)
        hotkeyWarningItem?.isHidden = true
    }

    func markHotkeyFailed() {
        statusItem.button?.title = ""
        let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .semibold)
        statusItem.button?.image = NSImage(systemSymbolName: "exclamationmark.triangle.fill", accessibilityDescription: "Hotkey failed")?
            .withSymbolConfiguration(config)
        if hotkeyWarningItem == nil {
            let item = NSMenuItem(title: "Hotkey not registered. Disable Spotlight's ⌘+Space.", action: nil, keyEquivalent: "")
            item.isEnabled = false
            menu.insertItem(item, at: 1)
            hotkeyWarningItem = item
        } else {
            hotkeyWarningItem?.isHidden = false
        }
    }

    private func configure() {
        menu.addItem(NSMenuItem(title: "Show Luma", action: #selector(show), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Settings...", action: #selector(settings), keyEquivalent: ","))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit Luma", action: #selector(quit), keyEquivalent: "q"))

        for item in menu.items {
            item.target = self
        }

        statusItem.menu = menu
    }

    @objc private func show() {
        onShow()
    }

    @objc private func settings() {
        onSettings()
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }
}
