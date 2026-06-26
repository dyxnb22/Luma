import AppKit
import LumaCore

@MainActor
final class LauncherActionPanel: NSView {
    private let stack = NSStackView()
    private var actionRows: [NSView] = []
    private var actions: [Action] = []
    private var item: ResultItem?
    private var selectedIndex = 0
    var onRun: ((Action, ResultItem) -> Void)?
    var onClose: (() -> Void)?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        wantsLayer = true
        layer?.cornerRadius = 12
        layer?.cornerCurve = .continuous
        layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.95).cgColor
        layer?.borderWidth = 1
        layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.35).cgColor

        stack.orientation = .vertical
        stack.spacing = 2
        stack.alignment = .leading
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)

        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor, constant: 10),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -10)
        ])
        isHidden = true
        alphaValue = 0
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var isVisible: Bool { !isHidden && alphaValue > 0.5 }
    var actionCount: Int { actions.count }

    func present(item: ResultItem, relativeTo anchor: NSView) {
        self.item = item
        actions = [item.primaryAction] + item.secondaryActions
        selectedIndex = 0
        rebuildRows(title: item.title)
        isHidden = false
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.12
            animator().alphaValue = 1
        }
        NSAccessibility.post(element: self, notification: .layoutChanged)
    }

    func dismiss() {
        guard isVisible else { return }
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.1
            animator().alphaValue = 0
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                self?.isHidden = true
                self?.onClose?()
            }
        }
    }

    func moveSelection(delta: Int) {
        guard !actions.isEmpty else { return }
        selectedIndex = min(max(0, selectedIndex + delta), actions.count - 1)
        updateHighlights()
    }

    func activateSelection() {
        guard let item, actions.indices.contains(selectedIndex) else { return }
        onRun?(actions[selectedIndex], item)
        dismiss()
    }

    func activateIndex(_ index: Int) {
        guard actions.indices.contains(index) else { return }
        selectedIndex = index
        activateSelection()
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        guard isVisible else { return false }
        switch event.keyCode {
        case 125: moveSelection(delta: 1); return true
        case 126: moveSelection(delta: -1); return true
        case 36: activateSelection(); return true
        case 53: dismiss(); return true
        case 48: dismiss(); return true
        default: return false
        }
    }

    private func rebuildRows(title: String) {
        stack.arrangedSubviews.forEach { view in
            stack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        actionRows.removeAll()

        let titleLabel = NSTextField(labelWithString: title)
        titleLabel.font = .systemFont(ofSize: 13, weight: .semibold)
        titleLabel.textColor = .labelColor
        titleLabel.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(titleLabel)

        for (index, action) in actions.enumerated() {
            let row = makeActionRow(action: action, index: index)
            actionRows.append(row)
            stack.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        }
        updateHighlights()
    }

    private func makeActionRow(action: Action, index: Int) -> NSView {
        let container = ActionRowHost(index: index)

        let label = NSTextField(labelWithString: action.title)
        label.font = .systemFont(ofSize: 13)
        label.textColor = .labelColor
        label.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(label)

        let shortcut = NSTextField(labelWithString: index < 9 ? "⌘\(index + 1)" : "")
        shortcut.font = .systemFont(ofSize: 11, weight: .medium)
        shortcut.textColor = .tertiaryLabelColor
        shortcut.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(shortcut)

        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 4),
            label.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            shortcut.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -4),
            shortcut.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            label.trailingAnchor.constraint(lessThanOrEqualTo: shortcut.leadingAnchor, constant: -8)
        ])
        return container
    }

    private func updateHighlights() {
        for (index, row) in actionRows.enumerated() {
            row.layer?.backgroundColor = index == selectedIndex
                ? NSColor.controlAccentColor.withAlphaComponent(0.08).cgColor
                : NSColor.clear.cgColor
            row.wantsLayer = true
            row.layer?.cornerRadius = 6
        }
    }
}

@MainActor
private final class ActionRowHost: NSView {
    let rowIndex: Int
    init(index: Int) {
        self.rowIndex = index
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 28).isActive = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}
