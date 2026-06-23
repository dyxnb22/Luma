import AppKit

@MainActor
final class SidebarMoreRow: NSControl {
    private let onExpand: () -> Void
    private var fullWidthConstraint: NSLayoutConstraint?
    private let titleLabel = NSTextField(labelWithString: "")

    init(label: String, onExpand: @escaping () -> Void) {
        self.onExpand = onExpand
        super.init(frame: .zero)
        wantsLayer = true
        layer?.cornerRadius = 8
        layer?.cornerCurve = .continuous
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 32).isActive = true

        titleLabel.stringValue = label
        titleLabel.font = .systemFont(ofSize: 12, weight: .medium)
        titleLabel.textColor = .secondaryLabelColor
        titleLabel.alignment = .center
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(titleLabel)
        NSLayoutConstraint.activate([
            titleLabel.centerXAnchor.constraint(equalTo: centerXAnchor),
            titleLabel.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])

        setAccessibilityRole(.button)
        setAccessibilityLabel(label)
        setAccessibilityHelp("Shows more open applications.")
    }

    override var acceptsFirstResponder: Bool { true }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func mouseUp(with event: NSEvent) {
        super.mouseUp(with: event)
        activate()
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 36 || event.keyCode == 76 || event.keyCode == 49 {
            activate()
            return
        }
        super.keyDown(with: event)
    }

    private func activate() {
        onExpand()
    }

    override func resetCursorRects() {
        addCursorRect(bounds, cursor: .pointingHand)
    }

    func updateLabel(_ label: String) {
        titleLabel.stringValue = label
        setAccessibilityLabel(label)
    }

    func bindFullWidth(to stack: NSStackView) {
        guard fullWidthConstraint == nil else { return }
        let constraint = widthAnchor.constraint(equalTo: stack.widthAnchor)
        constraint.isActive = true
        fullWidthConstraint = constraint
    }
}
