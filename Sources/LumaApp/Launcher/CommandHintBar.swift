import AppKit
import LumaCore

@MainActor
final class CommandHintBar: NSView {
    private let stack = NSStackView()
    private let formatLine = CommandHintBar.makeLine(mono: true)
    private let descriptionLine = CommandHintBar.makeLine(mono: false)
    private let exampleLine = CommandHintBar.makeLine(mono: true)
    private let returnLine = CommandHintBar.makeLine(mono: true)
    private let statusLine = CommandHintBar.makeLine(mono: false)
    private var heightConstraint: NSLayoutConstraint!
    private var statusDismissTask: Task<Void, Never>?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false

        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 2
        stack.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(formatLine)
        stack.addArrangedSubview(descriptionLine)
        stack.addArrangedSubview(exampleLine)
        stack.addArrangedSubview(returnLine)
        stack.addArrangedSubview(statusLine)
        addSubview(stack)

        returnLine.isHidden = true
        statusLine.isHidden = true
        statusLine.textColor = Self.hintTextColor

        heightConstraint = heightAnchor.constraint(equalToConstant: 0)
        heightConstraint.isActive = true

        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            stack.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -4),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(_ hint: CommandHint?, helpTrigger: String? = nil) {
        statusDismissTask?.cancel()
        statusLine.isHidden = true
        guard let hint else {
            collapse()
            formatLine.stringValue = ""
            descriptionLine.stringValue = ""
            exampleLine.stringValue = ""
            returnLine.stringValue = ""
            returnLine.isHidden = true
            return
        }

        isHidden = false
        heightConstraint.constant = LauncherChromeTokens.commandHintHeight
        formatLine.stringValue = "Format: \(hint.usageFormat)"
        if let helpTrigger, !helpTrigger.isEmpty, helpTrigger != "help", helpTrigger != "?" {
            descriptionLine.stringValue = "About: \(hint.description)\(L10n.tr("commandHint.helpSuffix", helpTrigger))"
        } else {
            descriptionLine.stringValue = "About: \(hint.description)"
        }
        if let example = hint.example {
            exampleLine.stringValue = "Example: \(example)"
            exampleLine.isHidden = false
        } else {
            exampleLine.stringValue = ""
            exampleLine.isHidden = true
        }
    }

    func setReturnAction(_ text: String?) {
        guard let text, !text.isEmpty else {
            returnLine.stringValue = ""
            returnLine.isHidden = true
            return
        }
        returnLine.stringValue = "Return: \(text)"
        returnLine.isHidden = false
    }

    func showStatus(_ message: String, duration: TimeInterval = 1.6) {
        statusDismissTask?.cancel()
        statusLine.stringValue = message
        statusLine.isHidden = false
        isHidden = false
        heightConstraint.constant = LauncherChromeTokens.commandHintHeight
        statusDismissTask = Task {
            try? await Task.sleep(for: .seconds(duration))
            guard !Task.isCancelled else { return }
            await MainActor.run {
                self.statusLine.isHidden = true
                self.statusLine.stringValue = ""
            }
        }
    }

    private func collapse() {
        heightConstraint.constant = 0
        isHidden = true
    }

    private static func makeLine(mono: Bool) -> NSTextField {
        let label = NSTextField(labelWithString: "")
        label.font = mono ? TypographyTokens.monoCaption() : TypographyTokens.caption2()
        label.textColor = mono ? Self.monoHintTextColor : Self.hintTextColor
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.lineBreakMode = .byTruncatingTail
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }

    private static var hintTextColor: NSColor {
        .labelColor.withAlphaComponent(0.72)
    }

    private static var monoHintTextColor: NSColor {
        .labelColor.withAlphaComponent(0.64)
    }
}
