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
        clipsToBounds = true
        setContentHuggingPriority(.defaultLow, for: .horizontal)
        setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

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
            stack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
        for label in [formatLine, descriptionLine, exampleLine, returnLine, statusLine] {
            label.widthAnchor.constraint(lessThanOrEqualTo: stack.widthAnchor).isActive = true
        }
    }

    override func layout() {
        super.layout()
        updateLabelWidths()
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
        updateLabelWidths()
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
        refreshHeight()
    }

    private func updateLabelWidths() {
        let maxWidth = bounds.width > 0 ? max(0, bounds.width - 8) : 640
        for label in [formatLine, descriptionLine, exampleLine, returnLine, statusLine] {
            label.preferredMaxLayoutWidth = maxWidth
        }
    }

    private func refreshHeight() {
        layoutSubtreeIfNeeded()
        let contentHeight = stack.fittingSize.height
        heightConstraint.constant = max(44, min(76, contentHeight + 6))
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
        updateLabelWidths()
        refreshHeight()
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
        label.maximumNumberOfLines = 2
        label.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        label.setContentHuggingPriority(.defaultLow, for: .horizontal)
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
