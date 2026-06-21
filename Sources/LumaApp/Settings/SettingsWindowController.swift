import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

struct SettingsSnapshot {
    var enabledModules: Set<ModuleIdentifier>
    var clipboardMaxEntries: Int
    var clipboardMaxAgeDays: Int
    var clipboardMaxEntrySizeKB: Int
    var translationTargetLanguage: String
    let modules: [(id: ModuleIdentifier, name: String)]
}

@MainActor
final class SettingsWindowController {
    private var window: NSWindow?
    private let config: ConfigurationStore
    private let onModulesChanged: @MainActor (Set<ModuleIdentifier>) -> Void

    init(config: ConfigurationStore, onModulesChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void) {
        self.config = config
        self.onModulesChanged = onModulesChanged
    }

    func show() {
        Task {
            let snapshot = await makeSnapshot()
            if let window {
                (window.contentView as? SettingsRootView)?.apply(snapshot)
                window.makeKeyAndOrderFront(nil)
                NSApp.activate(ignoringOtherApps: true)
                return
            }

            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 560, height: 520),
                styleMask: [.titled, .closable, .miniaturizable],
                backing: .buffered,
                defer: false
            )
            window.title = "Luma Settings"
            window.center()
            let root = SettingsRootView(snapshot: snapshot, config: config, onModulesChanged: onModulesChanged)
            window.contentView = root
            window.isReleasedWhenClosed = false
            self.window = window
            window.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
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
            modules: modules
        )
    }
}

@MainActor
final class SettingsRootView: NSView {
    private let config: ConfigurationStore
    private let onModulesChanged: (Set<ModuleIdentifier>) -> Void
    private let stack = NSStackView()
    private var moduleSwitches: [ModuleIdentifier: NSButton] = [:]
    private let maxEntriesField = NSTextField()
    private let maxDaysField = NSTextField()
    private let maxKBField = NSTextField()
    private let translationField = NSTextField()

    init(snapshot: SettingsSnapshot, config: ConfigurationStore, onModulesChanged: @escaping (Set<ModuleIdentifier>) -> Void) {
        self.config = config
        self.onModulesChanged = onModulesChanged
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
    }

    private func setup(snapshot: SettingsSnapshot) {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.translatesAutoresizingMaskIntoConstraints = false
        addSubview(scroll)

        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 14
        stack.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = stack

        stack.addArrangedSubview(sectionTitle("Luma Settings"))
        stack.addArrangedSubview(sectionTitle("Hotkey"))
        let hotkey = NSTextField(wrappingLabelWithString: "Command+Space. If registration fails, disable the Spotlight shortcut in System Settings.")
        hotkey.font = .systemFont(ofSize: 13)
        hotkey.textColor = .secondaryLabelColor
        stack.addArrangedSubview(hotkey)

        stack.addArrangedSubview(sectionTitle("Modules"))
        for module in snapshot.modules {
            let toggle = NSButton(checkboxWithTitle: module.name, target: self, action: #selector(moduleToggled(_:)))
            toggle.state = snapshot.enabledModules.contains(module.id) ? .on : .off
            toggle.identifier = NSUserInterfaceItemIdentifier(module.id.rawValue)
            moduleSwitches[module.id] = toggle
            stack.addArrangedSubview(toggle)
        }

        stack.addArrangedSubview(sectionTitle("Clipboard Retention"))
        configureNumericField(maxEntriesField, value: snapshot.clipboardMaxEntries, action: #selector(clipboardSettingsChanged))
        configureNumericField(maxDaysField, value: snapshot.clipboardMaxAgeDays, action: #selector(clipboardSettingsChanged))
        configureNumericField(maxKBField, value: snapshot.clipboardMaxEntrySizeKB, action: #selector(clipboardSettingsChanged))
        stack.addArrangedSubview(labeledField("Max entries", field: maxEntriesField))
        stack.addArrangedSubview(labeledField("Max age (days)", field: maxDaysField))
        stack.addArrangedSubview(labeledField("Max entry size (KB)", field: maxKBField))

        stack.addArrangedSubview(sectionTitle("Translation"))
        translationField.stringValue = snapshot.translationTargetLanguage
        translationField.placeholderString = "Target language (e.g. en, zh-Hans)"
        translationField.target = self
        translationField.action = #selector(translationTargetChanged)
        stack.addArrangedSubview(labeledField("Target language", field: translationField))

        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: topAnchor),
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            stack.widthAnchor.constraint(equalTo: scroll.widthAnchor, constant: -28)
        ])
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
        }
    }

    @objc private func translationTargetChanged() {
        let value = translationField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !value.isEmpty else { return }
        Task { await config.setTranslationTargetLanguage(value) }
    }

    private func sectionTitle(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = .systemFont(ofSize: 15, weight: .semibold)
        return label
    }

    private func labeledField(_ title: String, field: NSView) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 12
        let label = NSTextField(labelWithString: title)
        label.font = .systemFont(ofSize: 13)
        label.widthAnchor.constraint(equalToConstant: 160).isActive = true
        field.widthAnchor.constraint(equalToConstant: 120).isActive = true
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
