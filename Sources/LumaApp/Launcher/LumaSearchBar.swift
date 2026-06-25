import AppKit
import LumaCore
import LumaModules

@MainActor
final class LumaSearchBar: NSView {
    enum KeyCommand {
        case up
        case down
        case tab
        case actionPanel
        case commandNumber(Int)
    }

    private let iconView = NSImageView()
    private let textField = LumaSearchTextField()
    private let settingsButton = SettingsGearButton()
    private let hintsButton = NSButton()
    private let clearButton = NSButton()
    private let hintsPopover = NSPopover()
    var onTextChange: ((String) -> Void)?
    var onEscape: (() -> Void)?
    var onReturn: (() -> Void)?
    var onKeyCommand: ((KeyCommand) -> Bool)?
    var onDetailKey: ((NSEvent) -> Bool)?
    var onInterceptKeyDown: ((NSEvent) -> Bool)?
    var onOpenSettings: (() -> Void)?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false

        let symbolConfig = NSImage.SymbolConfiguration(pointSize: 16, weight: .medium)
        iconView.image = NSImage(systemSymbolName: "magnifyingglass", accessibilityDescription: nil)?
            .withSymbolConfiguration(symbolConfig)
        iconView.contentTintColor = .secondaryLabelColor
        iconView.imageScaling = .scaleProportionallyDown
        iconView.translatesAutoresizingMaskIntoConstraints = false

        textField.isBordered = false
        textField.isBezeled = false
        textField.drawsBackground = false
        textField.font = .systemFont(ofSize: 18, weight: .regular)
        textField.placeholderString = ModuleSearchHints.default
        textField.focusRingType = .none
        textField.delegate = self
        textField.target = self
        textField.action = #selector(submit)
        textField.translatesAutoresizingMaskIntoConstraints = false
        textField.onEscape = { [weak self] in self?.onEscape?() }
        textField.onKeyCommand = { [weak self] command in self?.onKeyCommand?(command) ?? false }
        textField.onDetailKey = { [weak self] event in self?.onDetailKey?(event) ?? false }
        textField.onInterceptKeyDown = { [weak self] event in self?.onInterceptKeyDown?(event) ?? false }

        hintsButton.isBordered = false
        hintsButton.bezelStyle = .inline
        hintsButton.image = NSImage(systemSymbolName: "info.circle", accessibilityDescription: "Keyboard shortcuts")
        hintsButton.imagePosition = .imageOnly
        hintsButton.contentTintColor = .tertiaryLabelColor
        hintsButton.target = self
        hintsButton.action = #selector(hintsTapped)
        hintsButton.toolTip = "Keyboard shortcuts"
        hintsButton.refusesFirstResponder = true
        hintsButton.translatesAutoresizingMaskIntoConstraints = false

        settingsButton.isBordered = false
        settingsButton.bezelStyle = .inline
        settingsButton.image = NSImage(systemSymbolName: "gearshape", accessibilityDescription: "Settings")
        settingsButton.imagePosition = .imageOnly
        settingsButton.contentTintColor = .tertiaryLabelColor
        settingsButton.target = self
        settingsButton.action = #selector(settingsTapped)
        settingsButton.toolTip = "Settings"
        settingsButton.refusesFirstResponder = true
        settingsButton.translatesAutoresizingMaskIntoConstraints = false

        clearButton.isBordered = false
        clearButton.bezelStyle = .inline
        clearButton.image = NSImage(systemSymbolName: "xmark.circle.fill", accessibilityDescription: "Clear search")
        clearButton.imagePosition = .imageOnly
        clearButton.contentTintColor = .tertiaryLabelColor
        clearButton.target = self
        clearButton.action = #selector(clearTapped)
        clearButton.isHidden = true
        clearButton.refusesFirstResponder = true
        clearButton.translatesAutoresizingMaskIntoConstraints = false

        addSubview(iconView)
        addSubview(textField)
        addSubview(hintsButton)
        addSubview(settingsButton)
        addSubview(clearButton)

        NSLayoutConstraint.activate([
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: 16),
            iconView.heightAnchor.constraint(equalToConstant: 16),
            textField.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 8),
            textField.trailingAnchor.constraint(equalTo: clearButton.leadingAnchor, constant: -6),
            textField.centerYAnchor.constraint(equalTo: centerYAnchor),
            settingsButton.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            settingsButton.centerYAnchor.constraint(equalTo: iconView.centerYAnchor),
            settingsButton.widthAnchor.constraint(equalToConstant: 22),
            settingsButton.heightAnchor.constraint(equalToConstant: 22),
            hintsButton.trailingAnchor.constraint(equalTo: settingsButton.leadingAnchor, constant: -8),
            hintsButton.centerYAnchor.constraint(equalTo: iconView.centerYAnchor),
            hintsButton.widthAnchor.constraint(equalToConstant: 22),
            hintsButton.heightAnchor.constraint(equalToConstant: 22),
            clearButton.trailingAnchor.constraint(equalTo: hintsButton.leadingAnchor, constant: -8),
            clearButton.centerYAnchor.constraint(equalTo: iconView.centerYAnchor),
            clearButton.widthAnchor.constraint(equalToConstant: 20),
            clearButton.heightAnchor.constraint(equalToConstant: 20)
        ])

        textField.nextKeyView = textField
        hintsButton.nextKeyView = textField
        settingsButton.nextKeyView = textField
        clearButton.nextKeyView = textField
        updateClearButtonVisibility()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var stringValue: String {
        get { textField.stringValue }
        set {
            textField.stringValue = newValue
            updateClearButtonVisibility()
        }
    }

    func setPlaceholder(_ text: String) {
        textField.placeholderString = text
    }

    func focus() {
        window?.makeFirstResponder(textField)
    }

    @objc private func submit() {
        onReturn?()
    }

    @objc private func clearTapped() {
        textField.stringValue = ""
        updateClearButtonVisibility()
        onTextChange?("")
        focus()
    }

    @objc private func settingsTapped() {
        onOpenSettings?()
    }

    @objc private func hintsTapped() {
        if hintsPopover.isShown {
            hintsPopover.performClose(self)
            return
        }
        let label = NSTextField(wrappingLabelWithString: """
        ⌘1…9 — jump to row
        ↑ ↓ — move selection
        Tab / ⌘K — action panel
        Return — run selected item
        Esc — back / close panel
        """)
        label.font = .systemFont(ofSize: 12)
        label.textColor = .labelColor
        label.preferredMaxLayoutWidth = 220
        let host = NSView(frame: NSRect(x: 0, y: 0, width: 240, height: 120))
        label.translatesAutoresizingMaskIntoConstraints = false
        host.addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: host.topAnchor, constant: 12),
            label.leadingAnchor.constraint(equalTo: host.leadingAnchor, constant: 12),
            label.trailingAnchor.constraint(equalTo: host.trailingAnchor, constant: -12),
            label.bottomAnchor.constraint(equalTo: host.bottomAnchor, constant: -12)
        ])
        hintsPopover.contentSize = NSSize(width: 240, height: 120)
        hintsPopover.contentViewController = NSViewController()
        hintsPopover.contentViewController?.view = host
        hintsPopover.behavior = .transient
        hintsPopover.show(relativeTo: hintsButton.bounds, of: hintsButton, preferredEdge: .maxY)
    }

    private func updateClearButtonVisibility() {
        clearButton.isHidden = textField.stringValue.isEmpty
    }
}

extension LumaSearchBar: NSTextFieldDelegate {
    func controlTextDidChange(_ obj: Notification) {
        let now = CFAbsoluteTimeGetCurrent()
        LatencyTracker.shared.markKeystroke(at: now)
        updateClearButtonVisibility()
        onTextChange?(textField.stringValue)
    }
}

@MainActor
final class LatencyTracker {
    static let shared = LatencyTracker()
    private var lastKeystroke: CFAbsoluteTime?

    func markKeystroke(at time: CFAbsoluteTime) {
        lastKeystroke = time
    }

    func markFirstPaint() -> Double? {
        guard let last = lastKeystroke else { return nil }
        lastKeystroke = nil
        return (CFAbsoluteTimeGetCurrent() - last) * 1000
    }
}

@MainActor
private final class LumaSearchTextField: NSTextField {
    var onEscape: (() -> Void)?
    var onKeyCommand: ((LumaSearchBar.KeyCommand) -> Bool)?
    var onDetailKey: ((NSEvent) -> Bool)?
    var onInterceptKeyDown: ((NSEvent) -> Bool)?

    override func keyDown(with event: NSEvent) {
        if onInterceptKeyDown?(event) == true { return }
        if stringValue.isEmpty, onDetailKey?(event) == true { return }
        if event.keyCode == 125, onKeyCommand?(.down) == true { return }
        if event.keyCode == 126, onKeyCommand?(.up) == true { return }
        if event.keyCode == 48, onKeyCommand?(.tab) == true { return }
        if event.modifierFlags.contains(.command),
           event.charactersIgnoringModifiers?.lowercased() == "k",
           onKeyCommand?(.actionPanel) == true { return }
        if event.modifierFlags.contains(.command),
           let chars = event.charactersIgnoringModifiers,
           let number = Int(chars),
           (1...9).contains(number),
           onKeyCommand?(.commandNumber(number)) == true {
            return
        }
        super.keyDown(with: event)
    }

    override func cancelOperation(_ sender: Any?) {
        onEscape?()
    }

    override func insertTab(_ sender: Any?) {
        if onKeyCommand?(.tab) == true { return }
    }

    override func insertBacktab(_ sender: Any?) {
        if onKeyCommand?(.tab) == true { return }
    }
}

@MainActor
private final class SettingsGearButton: NSButton {
    private var tracking: NSTrackingArea?

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking { removeTrackingArea(tracking) }
        let area = NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        tracking = area
    }

    override func mouseEntered(with event: NSEvent) {
        contentTintColor = .secondaryLabelColor
    }

    override func mouseExited(with event: NSEvent) {
        contentTintColor = .tertiaryLabelColor
    }
}
