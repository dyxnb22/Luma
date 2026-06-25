import AppKit
import LumaCore

@MainActor
final class CommandHintBar: NSView {
    private let triggerLabel = NSTextField(labelWithString: "")
    private let titleLabel = NSTextField(labelWithString: "")
    private let exampleLabel = NSTextField(labelWithString: "")
    private var heightConstraint: NSLayoutConstraint!

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false

        triggerLabel.font = GeekStyleTokens.mono(size: 11, weight: .semibold)
        triggerLabel.textColor = .secondaryLabelColor
        triggerLabel.isBezeled = false
        triggerLabel.isEditable = false
        triggerLabel.drawsBackground = false
        triggerLabel.translatesAutoresizingMaskIntoConstraints = false
        triggerLabel.setContentHuggingPriority(.required, for: .horizontal)

        titleLabel.font = .systemFont(ofSize: 11, weight: .medium)
        titleLabel.textColor = .secondaryLabelColor
        titleLabel.isBezeled = false
        titleLabel.isEditable = false
        titleLabel.drawsBackground = false
        titleLabel.translatesAutoresizingMaskIntoConstraints = false
        titleLabel.lineBreakMode = .byTruncatingTail

        exampleLabel.font = GeekStyleTokens.mono(size: 11)
        exampleLabel.textColor = .tertiaryLabelColor
        exampleLabel.alignment = .right
        exampleLabel.isBezeled = false
        exampleLabel.isEditable = false
        exampleLabel.drawsBackground = false
        exampleLabel.translatesAutoresizingMaskIntoConstraints = false
        exampleLabel.lineBreakMode = .byTruncatingTail

        addSubview(triggerLabel)
        addSubview(titleLabel)
        addSubview(exampleLabel)

        heightConstraint = heightAnchor.constraint(equalToConstant: 0)
        heightConstraint.isActive = true

        NSLayoutConstraint.activate([
            triggerLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            triggerLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            titleLabel.leadingAnchor.constraint(equalTo: triggerLabel.trailingAnchor, constant: 10),
            titleLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            exampleLabel.leadingAnchor.constraint(greaterThanOrEqualTo: titleLabel.trailingAnchor, constant: 12),
            exampleLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            exampleLabel.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(_ hint: CommandHint?) {
        guard let hint else {
            heightConstraint.constant = 0
            triggerLabel.stringValue = ""
            titleLabel.stringValue = ""
            exampleLabel.stringValue = ""
            exampleLabel.isHidden = true
            return
        }
        heightConstraint.constant = 20
        triggerLabel.stringValue = hint.trigger
        titleLabel.stringValue = hint.title
        exampleLabel.stringValue = hint.example ?? ""
        exampleLabel.isHidden = hint.example == nil
    }
}
