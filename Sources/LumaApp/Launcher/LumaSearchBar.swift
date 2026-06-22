import AppKit

@MainActor
final class LumaSearchBar: NSView {
    enum KeyCommand {
        case up
        case down
        case tab
        case commandNumber(Int)
    }

    private let iconView = NSImageView()
    private let textField = LumaSearchTextField()
    var onTextChange: ((String) -> Void)?
    var onEscape: (() -> Void)?
    var onReturn: (() -> Void)?
    var onKeyCommand: ((KeyCommand) -> Bool)?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 52).isActive = true

        let symbolConfig = NSImage.SymbolConfiguration(pointSize: 16, weight: .medium)
        iconView.image = NSImage(systemSymbolName: "magnifyingglass", accessibilityDescription: nil)?
            .withSymbolConfiguration(symbolConfig)
        iconView.contentTintColor = .secondaryLabelColor
        iconView.imageScaling = .scaleProportionallyDown
        iconView.translatesAutoresizingMaskIntoConstraints = false

        textField.isBordered = false
        textField.isBezeled = false
        textField.drawsBackground = false
        textField.font = .systemFont(ofSize: 20, weight: .regular)
        textField.placeholderString = "Search"
        textField.focusRingType = .none
        textField.delegate = self
        textField.target = self
        textField.action = #selector(submit)
        textField.translatesAutoresizingMaskIntoConstraints = false
        textField.onEscape = { [weak self] in self?.onEscape?() }
        textField.onKeyCommand = { [weak self] command in self?.onKeyCommand?(command) ?? false }

        addSubview(iconView)
        addSubview(textField)

        NSLayoutConstraint.activate([
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 18),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: 16),
            iconView.heightAnchor.constraint(equalToConstant: 16),
            textField.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 10),
            textField.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -18),
            textField.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var stringValue: String {
        get { textField.stringValue }
        set { textField.stringValue = newValue }
    }

    func focus() {
        window?.makeFirstResponder(textField)
    }

    @objc private func submit() {
        onReturn?()
    }
}

extension LumaSearchBar: NSTextFieldDelegate {
    func controlTextDidChange(_ obj: Notification) {
        let now = CFAbsoluteTimeGetCurrent()
        LatencyTracker.shared.markKeystroke(at: now)
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

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 125, onKeyCommand?(.down) == true { return }
        if event.keyCode == 126, onKeyCommand?(.up) == true { return }
        if event.keyCode == 48, onKeyCommand?(.tab) == true { return }
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
        // Esc is handled here only; do not also handle keyCode 53 in keyDown to avoid double-fire.
        onEscape?()
    }
}
