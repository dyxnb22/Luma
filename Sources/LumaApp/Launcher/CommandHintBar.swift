import AppKit
import LumaCore

@MainActor
final class CommandHintBar: NSView {
    private let stack = NSStackView()
    private let formatLine = CommandHintBar.makeLine(mono: true)
    private let descriptionLine = CommandHintBar.makeLine(mono: false)
    private let exampleLine = CommandHintBar.makeLine(mono: true)
    private var heightConstraint: NSLayoutConstraint!

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
        addSubview(stack)

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

    func apply(_ hint: CommandHint?) {
        guard let hint else {
            heightConstraint.constant = 0
            isHidden = true
            formatLine.stringValue = ""
            descriptionLine.stringValue = ""
            exampleLine.stringValue = ""
            return
        }

        isHidden = false
        heightConstraint.constant = LauncherChromeTokens.commandHintHeight
        formatLine.stringValue = "Format: \(hint.usageFormat)"
        descriptionLine.stringValue = "About: \(hint.description)"
        if let example = hint.example {
            exampleLine.stringValue = "Example: \(example)"
            exampleLine.isHidden = false
        } else {
            exampleLine.stringValue = ""
            exampleLine.isHidden = true
        }
    }

    private static func makeLine(mono: Bool) -> NSTextField {
        let label = NSTextField(labelWithString: "")
        label.font = mono ? TypographyTokens.monoCaption() : TypographyTokens.caption2()
        label.textColor = mono ? .tertiaryLabelColor : .secondaryLabelColor
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.lineBreakMode = .byTruncatingTail
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }
}
