import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

struct SettingsSnapshot {
    var enabledModules: Set<ModuleIdentifier>
    var clipboardMaxEntries: Int
    var clipboardMaxAgeDays: Int
    var clipboardMaxEntrySizeKB: Int
    var translationTargetLanguage: String
    var secretsAutoClearSeconds: Int
    var secretsRelockTimeoutSeconds: Int
    var secretsRequireUnlockOnLaunch: Bool
    var latencyHUDEnabled: Bool
    let modules: [(id: ModuleIdentifier, name: String)]
}

@MainActor
final class SettingsWindowController {
    private var window: NSWindow?
    private let config: ConfigurationStore
    private let usage: PersistentUsageTracker
    private let onModulesChanged: @MainActor (Set<ModuleIdentifier>) -> Void
    private let onClipboardSettingsChanged: @MainActor (Int, Int, Int) -> Void
    private let onSecretsSettingsChanged: @MainActor (Int, Int) -> Void
    private let onLatencyHUDChanged: @MainActor (Bool) -> Void

    init(
        config: ConfigurationStore,
        usage: PersistentUsageTracker,
        onModulesChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void,
        onClipboardSettingsChanged: @escaping @MainActor (Int, Int, Int) -> Void,
        onSecretsSettingsChanged: @escaping @MainActor (Int, Int) -> Void,
        onLatencyHUDChanged: @escaping @MainActor (Bool) -> Void
    ) {
        self.config = config
        self.usage = usage
        self.onModulesChanged = onModulesChanged
        self.onClipboardSettingsChanged = onClipboardSettingsChanged
        self.onSecretsSettingsChanged = onSecretsSettingsChanged
        self.onLatencyHUDChanged = onLatencyHUDChanged
    }

    func show() {
        Task {
            let snapshot = await makeSnapshot()
            if let window {
                (window.contentView as? SettingsRootView)?.apply(snapshot)
                window.makeKeyAndOrderFront(nil)
                NSApp.activate()
                return
            }

            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 580, height: 520),
                styleMask: [.titled, .closable, .miniaturizable],
                backing: .buffered,
                defer: false
            )
            window.title = "Luma Settings"
            window.center()
            let root = SettingsRootView(
                snapshot: snapshot,
                config: config,
                usage: usage,
                onModulesChanged: onModulesChanged,
                onClipboardSettingsChanged: onClipboardSettingsChanged,
                onSecretsSettingsChanged: onSecretsSettingsChanged,
                onLatencyHUDChanged: onLatencyHUDChanged
            )
            window.contentView = root
            window.isReleasedWhenClosed = false
            self.window = window
            window.makeKeyAndOrderFront(nil)
            NSApp.activate()
        }
    }

    private func makeSnapshot() async -> SettingsSnapshot {
        let modules = BuiltInModules.makeAll().map { (type(of: $0).manifest.identifier, type(of: $0).manifest.displayName) }
        let defaultEnabled = Set(BuiltInModules.makeAll().filter { type(of: $0).manifest.defaultEnabled }.map { type(of: $0).manifest.identifier })
        return SettingsSnapshot(
            enabledModules: await config.enabledModules() ?? defaultEnabled,
            clipboardMaxEntries: await config.clipboardMaxEntries(),
            clipboardMaxAgeDays: await config.clipboardMaxAgeDays(),
            clipboardMaxEntrySizeKB: await config.clipboardMaxEntrySizeKB(),
            translationTargetLanguage: await config.translationTargetLanguage(),
            secretsAutoClearSeconds: await config.secretsAutoClearSeconds(),
            secretsRelockTimeoutSeconds: await config.secretsRelockTimeoutSeconds(),
            secretsRequireUnlockOnLaunch: await config.secretsRequireUnlockOnLaunch(),
            latencyHUDEnabled: await config.latencyHUDEnabled(),
            modules: modules
        )
    }
}

@MainActor
final class SettingsRootView: NSView, NSTabViewDelegate {
    private let config: ConfigurationStore
    private let usage: PersistentUsageTracker
    private let onModulesChanged: (Set<ModuleIdentifier>) -> Void
    private let onClipboardSettingsChanged: (Int, Int, Int) -> Void
    private let onSecretsSettingsChanged: (Int, Int) -> Void
    private let onLatencyHUDChanged: (Bool) -> Void
    private let tabView = NSTabView()
    private var moduleSwitches: [ModuleIdentifier: NSButton] = [:]
    private let maxEntriesField = NSTextField()
    private let maxDaysField = NSTextField()
    private let maxKBField = NSTextField()
    private let translationField = NSTextField()
    private let secretsClearField = NSTextField()
    private let secretsRelockField = NSTextField()
    private let secretsRequireUnlockToggle = NSButton()
    private let latencyHUDToggle = NSButton()
    private var activityView: ActivitySettingsView?

    init(
        snapshot: SettingsSnapshot,
        config: ConfigurationStore,
        usage: PersistentUsageTracker,
        onModulesChanged: @escaping (Set<ModuleIdentifier>) -> Void,
        onClipboardSettingsChanged: @escaping (Int, Int, Int) -> Void,
        onSecretsSettingsChanged: @escaping (Int, Int) -> Void,
        onLatencyHUDChanged: @escaping (Bool) -> Void
    ) {
        self.config = config
        self.usage = usage
        self.onModulesChanged = onModulesChanged
        self.onClipboardSettingsChanged = onClipboardSettingsChanged
        self.onSecretsSettingsChanged = onSecretsSettingsChanged
        self.onLatencyHUDChanged = onLatencyHUDChanged
        super.init(frame: .zero)
        setup(snapshot: snapshot)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(_ snapshot: SettingsSnapshot) {
        for (id, button) in moduleSwitches {
            button.state = snapshot.enabledModules.contains(id) ? .on : .off
        }
        maxEntriesField.stringValue = "\(snapshot.clipboardMaxEntries)"
        maxDaysField.stringValue = "\(snapshot.clipboardMaxAgeDays)"
        maxKBField.stringValue = "\(snapshot.clipboardMaxEntrySizeKB)"
        translationField.stringValue = snapshot.translationTargetLanguage
        secretsClearField.stringValue = "\(snapshot.secretsAutoClearSeconds)"
        secretsRelockField.stringValue = "\(snapshot.secretsRelockTimeoutSeconds)"
        secretsRequireUnlockToggle.state = snapshot.secretsRequireUnlockOnLaunch ? .on : .off
        latencyHUDToggle.state = snapshot.latencyHUDEnabled ? .on : .off
        activityView?.refresh()
    }

    private func setup(snapshot: SettingsSnapshot) {
        tabView.translatesAutoresizingMaskIntoConstraints = false
        tabView.delegate = self
        addSubview(tabView)

        tabView.addTabViewItem(tab("General", view: makeGeneralTab(snapshot: snapshot)))
        tabView.addTabViewItem(tab("Modules", view: makeModulesTab(snapshot: snapshot)))
        tabView.addTabViewItem(tab("Clipboard", view: makeClipboardTab(snapshot: snapshot)))
        tabView.addTabViewItem(tab("Translation", view: makeTranslationTab(snapshot: snapshot)))
        tabView.addTabViewItem(tab("Secrets", view: makeSecretsTab(snapshot: snapshot)))
        tabView.addTabViewItem(tab("Accessibility", view: makeAccessibilityTab()))
        tabView.addTabViewItem(tab("Activity", view: makeActivityTab()))

        NSLayoutConstraint.activate([
            tabView.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            tabView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            tabView.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            tabView.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -8)
        ])
    }

    func tabView(_ tabView: NSTabView, didSelect tabViewItem: NSTabViewItem?) {
        if tabViewItem?.label == "Activity" {
            activityView?.refresh()
        }
    }

    private func tab(_ label: String, view: NSView) -> NSTabViewItem {
        let item = NSTabViewItem(identifier: label)
        item.label = label
        item.view = view
        return item
    }

    private func makeGeneralTab(snapshot: SettingsSnapshot) -> NSView {
        let stack = verticalStack()
        stack.addArrangedSubview(sectionTitle("Hotkey"))
        let hotkeyHint = NSTextField(wrappingLabelWithString: "Command+Space is Luma's launcher hotkey. Disable Spotlight's shortcut if registration is blocked.")
        hotkeyHint.font = TypographyTokens.caption()
        hotkeyHint.textColor = .secondaryLabelColor
        stack.addArrangedSubview(hotkeyHint)

        stack.addArrangedSubview(sectionTitle("Developer"))
        latencyHUDToggle.setButtonType(.switch)
        latencyHUDToggle.title = "Show latency HUD overlay"
        latencyHUDToggle.state = snapshot.latencyHUDEnabled ? .on : .off
        latencyHUDToggle.target = self
        latencyHUDToggle.action = #selector(latencyHUDChanged)
        stack.addArrangedSubview(latencyHUDToggle)

        return padded(stack)
    }

    private func makeModulesTab(snapshot: SettingsSnapshot) -> NSView {
        let stack = verticalStack()
        stack.addArrangedSubview(sectionTitle("Enabled modules"))
        for module in snapshot.modules {
            let toggle = NSButton(checkboxWithTitle: module.name, target: self, action: #selector(moduleToggled(_:)))
            toggle.state = snapshot.enabledModules.contains(module.id) ? .on : .off
            toggle.identifier = NSUserInterfaceItemIdentifier(module.id.rawValue)
            moduleSwitches[module.id] = toggle
            stack.addArrangedSubview(toggle)
        }
        return padded(stack)
    }

    private func makeClipboardTab(snapshot: SettingsSnapshot) -> NSView {
        let stack = verticalStack()
        configureNumericField(maxEntriesField, value: snapshot.clipboardMaxEntries, action: #selector(clipboardSettingsChanged))
        configureNumericField(maxDaysField, value: snapshot.clipboardMaxAgeDays, action: #selector(clipboardSettingsChanged))
        configureNumericField(maxKBField, value: snapshot.clipboardMaxEntrySizeKB, action: #selector(clipboardSettingsChanged))
        stack.addArrangedSubview(labeledField("Max entries", field: maxEntriesField))
        stack.addArrangedSubview(labeledField("TTL (days)", field: maxDaysField))
        stack.addArrangedSubview(labeledField("Max entry size (KB)", field: maxKBField))
        return padded(stack)
    }

    private func makeTranslationTab(snapshot: SettingsSnapshot) -> NSView {
        let stack = verticalStack()
        translationField.stringValue = snapshot.translationTargetLanguage
        translationField.placeholderString = "Target language (e.g. en, zh-Hans)"
        translationField.target = self
        translationField.action = #selector(translationTargetChanged)
        stack.addArrangedSubview(labeledField("Target language", field: translationField))
        return padded(stack)
    }

    private func makeSecretsTab(snapshot: SettingsSnapshot) -> NSView {
        let stack = verticalStack()
        configureNumericField(secretsClearField, value: snapshot.secretsAutoClearSeconds, action: #selector(secretsSettingsChanged))
        configureNumericField(secretsRelockField, value: snapshot.secretsRelockTimeoutSeconds, action: #selector(secretsSettingsChanged))
        stack.addArrangedSubview(labeledField("Auto-clear pasteboard (s)", field: secretsClearField))
        stack.addArrangedSubview(labeledField("Re-lock timeout (s)", field: secretsRelockField))
        secretsRequireUnlockToggle.setButtonType(.switch)
        secretsRequireUnlockToggle.title = "Require unlock on launch"
        secretsRequireUnlockToggle.state = snapshot.secretsRequireUnlockOnLaunch ? .on : .off
        secretsRequireUnlockToggle.target = self
        secretsRequireUnlockToggle.action = #selector(secretsRequireUnlockChanged)
        stack.addArrangedSubview(secretsRequireUnlockToggle)
        return padded(stack)
    }

    private func makeAccessibilityTab() -> NSView {
        let stack = verticalStack()
        let axRow = NSStackView()
        axRow.orientation = .horizontal
        axRow.spacing = 10
        let axLabel = NSTextField(labelWithString: AXService.isProcessTrusted() ? "Granted ✓" : "Not granted")
        axLabel.font = TypographyTokens.body
        axLabel.textColor = AXService.isProcessTrusted() ? .systemGreen : .secondaryLabelColor
        let axButton = NSButton(title: "Open System Settings", target: self, action: #selector(openAXSettings))
        axButton.bezelStyle = .rounded
        axRow.addArrangedSubview(axLabel)
        axRow.addArrangedSubview(axButton)
        stack.addArrangedSubview(axRow)
        return padded(stack)
    }

    private func makeActivityTab() -> NSView {
        let view = ActivitySettingsView(usage: usage)
        activityView = view
        return padded(view)
    }

    private func verticalStack() -> NSStackView {
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12
        stack.translatesAutoresizingMaskIntoConstraints = false
        return stack
    }

    private func padded(_ content: NSView) -> NSView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        content.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = content
        NSLayoutConstraint.activate([
            content.leadingAnchor.constraint(equalTo: scroll.contentView.leadingAnchor, constant: 12),
            content.trailingAnchor.constraint(equalTo: scroll.contentView.trailingAnchor, constant: -12),
            content.topAnchor.constraint(equalTo: scroll.contentView.topAnchor, constant: 12),
            content.widthAnchor.constraint(equalTo: scroll.widthAnchor, constant: -24)
        ])
        return scroll
    }

    @objc private func latencyHUDChanged() {
        let enabled = latencyHUDToggle.state == .on
        Task {
            await config.setLatencyHUDEnabled(enabled)
            await MainActor.run { onLatencyHUDChanged(enabled) }
        }
    }

    @objc private func moduleToggled(_ sender: NSButton) {
        let enabled = Set(moduleSwitches.compactMap { id, button in button.state == .on ? id : nil })
        Task {
            await config.setEnabledModules(enabled)
            onModulesChanged(enabled)
        }
    }

    @objc private func clipboardSettingsChanged() {
        guard let entries = Int(maxEntriesField.stringValue),
              let days = Int(maxDaysField.stringValue),
              let kb = Int(maxKBField.stringValue),
              entries > 0, days > 0, kb > 0 else { return }
        Task {
            await config.setClipboardMaxEntries(entries)
            await config.setClipboardMaxAgeDays(days)
            await config.setClipboardMaxEntrySizeKB(kb)
            await MainActor.run {
                onClipboardSettingsChanged(entries, days, kb)
            }
        }
    }

    @objc private func secretsSettingsChanged() {
        guard let autoClear = Int(secretsClearField.stringValue),
              let relock = Int(secretsRelockField.stringValue),
              autoClear > 0, relock >= 30 else { return }
        Task {
            await config.setSecretsAutoClearSeconds(autoClear)
            await config.setSecretsRelockTimeoutSeconds(relock)
            await MainActor.run {
                onSecretsSettingsChanged(autoClear, relock)
            }
        }
    }

    @objc private func secretsRequireUnlockChanged() {
        Task {
            await config.setSecretsRequireUnlockOnLaunch(secretsRequireUnlockToggle.state == .on)
        }
    }

    @objc private func translationTargetChanged() {
        let value = translationField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !value.isEmpty else { return }
        Task { await config.setTranslationTargetLanguage(value) }
    }

    @objc private func openAXSettings() {
        AXService.requestPermission()
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
    }

    private func sectionTitle(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = TypographyTokens.title3
        return label
    }

    private func labeledField(_ title: String, field: NSView) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 12
        let label = NSTextField(labelWithString: title)
        label.font = TypographyTokens.body
        label.widthAnchor.constraint(equalToConstant: 180).isActive = true
        field.widthAnchor.constraint(equalToConstant: 140).isActive = true
        row.addArrangedSubview(label)
        row.addArrangedSubview(field)
        return row
    }

    private func configureNumericField(_ field: NSTextField, value: Int, action: Selector) {
        field.stringValue = "\(value)"
        field.target = self
        field.action = action
    }
}
