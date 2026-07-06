import AppKit

@MainActor
final class MenuBarController {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let menu = NSMenu()
    private var hotkeyWarningItem: NSMenuItem?
    private var hotkeyOK = true
    private var secretsVaultLocked = true
    private let onShow: @MainActor () -> Void
    private let onSettings: @MainActor () -> Void
    private let onRunDoctor: @MainActor () -> Void
    private let onExportDiagnostics: @MainActor () -> Void

    init(
        onShow: @escaping @MainActor () -> Void,
        onSettings: @escaping @MainActor () -> Void,
        onRunDoctor: @escaping @MainActor () -> Void,
        onExportDiagnostics: @escaping @MainActor () -> Void
    ) {
        self.onShow = onShow
        self.onSettings = onSettings
        self.onRunDoctor = onRunDoctor
        self.onExportDiagnostics = onExportDiagnostics
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

    private func refreshStatusIcon() {
        let state: LumaMenuBarIcon.State
        if !hotkeyOK {
            state = .hotkeyWarning
        } else if secretsVaultLocked {
            state = .vaultLocked
        } else {
            state = .vaultUnlocked
        }

        statusItem.length = NSStatusItem.squareLength
        statusItem.button?.image = LumaMenuBarIcon.make(state: state)
        statusItem.button?.title = ""
    }

    private func configure() {
        menu.addItem(NSMenuItem(title: "Show Luma", action: #selector(show), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Settings...", action: #selector(settings), keyEquivalent: ","))
        menu.addItem(NSMenuItem(title: "Run Doctor…", action: #selector(runDoctor), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Export Diagnostics…", action: #selector(exportDiagnostics), keyEquivalent: ""))
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

    @objc private func runDoctor() {
        onRunDoctor()
    }

    @objc private func exportDiagnostics() {
        onExportDiagnostics()
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
