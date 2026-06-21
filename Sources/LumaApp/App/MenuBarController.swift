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
    }

    func update(hotkeyOK: Bool) {
        statusItem.button?.title = hotkeyOK ? "L" : "⚠"
        if hotkeyOK {
            hotkeyWarningItem?.isHidden = true
        } else if hotkeyWarningItem == nil {
            let item = NSMenuItem(title: "Hotkey blocked (Spotlight?)", action: nil, keyEquivalent: "")
            item.isEnabled = false
            menu.insertItem(item, at: 1)
            hotkeyWarningItem = item
        } else {
            hotkeyWarningItem?.isHidden = false
        }
    }

    private func configure() {
        statusItem.button?.title = "L"

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
