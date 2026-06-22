import AppKit

@MainActor
final class MenuBarController {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let menu = NSMenu()
    private var hotkeyWarningItem: NSMenuItem?
    private var hotkeyOK = true
    private var secretsVaultLocked = true
    private var wordbookDueCount = 0
    private var todoDueCount = 0
    private let onShow: @MainActor () -> Void
    private let onSettings: @MainActor () -> Void

    init(onShow: @escaping @MainActor () -> Void, onSettings: @escaping @MainActor () -> Void) {
        self.onShow = onShow
        self.onSettings = onSettings
        configure()
        markHotkeyOK()
    }

    func markHotkeyOK() {
        hotkeyOK = true
        hotkeyWarningItem?.isHidden = true
        refreshStatusIcon()
    }

    func markHotkeyFailed() {
        hotkeyOK = false
        if hotkeyWarningItem == nil {
            let item = NSMenuItem(title: "Hotkey not registered. Disable Spotlight's ⌘+Space.", action: nil, keyEquivalent: "")
            item.isEnabled = false
            menu.insertItem(item, at: 1)
            hotkeyWarningItem = item
        } else {
            hotkeyWarningItem?.isHidden = false
        }
        refreshStatusIcon()
    }

    func setSecretsLockState(locked: Bool) {
        secretsVaultLocked = locked
        refreshStatusIcon()
    }

    func setDueCounts(wordbook: Int, todo: Int) {
        wordbookDueCount = max(0, wordbook)
        todoDueCount = max(0, todo)
        refreshStatusIcon()
    }

    private func refreshStatusIcon() {
        statusItem.button?.title = ""
        let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .semibold)
        let symbolName: String
        if !hotkeyOK {
            symbolName = "exclamationmark.triangle.fill"
        } else if secretsVaultLocked {
            symbolName = "lock.shield.fill"
        } else {
            symbolName = "lock.open.fill"
        }
        statusItem.button?.image = NSImage(systemSymbolName: symbolName, accessibilityDescription: "Luma")?
            .withSymbolConfiguration(config)

        let dueTotal = wordbookDueCount + todoDueCount
        if dueTotal > 0, hotkeyOK {
            statusItem.button?.title = " \(dueTotal)"
            statusItem.button?.font = .monospacedDigitSystemFont(ofSize: 11, weight: .bold)
        }
    }

    private func configure() {
        menu.addItem(NSMenuItem(title: "Show Luma", action: #selector(show), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Settings...", action: #selector(settings), keyEquivalent: ","))
        menu.addItem(NSMenuItem(title: "About Luma", action: #selector(showAbout), keyEquivalent: ""))
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

    @objc private func showAbout() {
        let alert = NSAlert()
        alert.messageText = "Luma"
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.1.0"
        alert.informativeText = "Version \(version)\n\nNative macOS launcher. Data stored locally at ~/Library/Application Support/Luma."
        alert.alertStyle = .informational
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }
}
