@preconcurrency import AppKit
import LumaCore

@MainActor
final class LauncherActionPanel: NSView {
    private let chromeView = NSView()
    private let stack = NSStackView()
    private var actionRows: [NSView] = []
    private var actions: [Action] = []
    private var item: ResultItem?
    private var selectedIndex = 0
    private weak var layoutRoot: NSView?
    private weak var fallbackBottomAnchor: NSView?
    private var bottomConstraint: NSLayoutConstraint?
    private let positioningGap: CGFloat = 8
    var onRun: ((Action, ResultItem) -> Void)?
    var onClose: (() -> Void)?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false

        chromeView.translatesAutoresizingMaskIntoConstraints = false
        chromeView.wantsLayer = true
        chromeView.layer?.cornerRadius = 12
        chromeView.layer?.cornerCurve = .continuous
        chromeView.layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.95).cgColor
        chromeView.layer?.borderWidth = 1
        chromeView.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.35).cgColor
        addSubview(chromeView)
        NSLayoutConstraint.activate([
            chromeView.topAnchor.constraint(equalTo: topAnchor),
            chromeView.leadingAnchor.constraint(equalTo: leadingAnchor),
            chromeView.trailingAnchor.constraint(equalTo: trailingAnchor),
            chromeView.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])

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

    func configureLayout(in root: NSView, fallbackBottomAnchor hintBar: NSView) {
        layoutRoot = root
        fallbackBottomAnchor = hintBar
        bottomConstraint?.isActive = false
        let hintTop = hintBar.convert(hintBar.bounds, to: root).maxY
        let initialBottomY = hintTop + positioningGap
        bottomConstraint = bottomAnchor.constraint(
            equalTo: root.bottomAnchor,
            constant: -(root.bounds.height - initialBottomY)
        )
        bottomConstraint?.isActive = true
    }

    func present(item: ResultItem, relativeTo anchor: NSView) {
        self.item = item
        actions = [item.primaryAction] + item.secondaryActions
        selectedIndex = 0
        rebuildRows(title: item.title)
        reposition(relativeTo: anchor)
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

    private func reposition(relativeTo anchor: NSView) {
        guard let root = layoutRoot, let fallback = fallbackBottomAnchor else { return }
        root.layoutSubtreeIfNeeded()
        let anchorView = anchor.window != nil ? anchor : fallback
        let anchorTop = anchorView.convert(anchorView.bounds, to: root).maxY
        let hintTop = fallback.convert(fallback.bounds, to: root).maxY
        let targetBottomY = max(anchorTop + positioningGap, hintTop + positioningGap)
        bottomConstraint?.constant = -(root.bounds.height - targetBottomY)
        root.layoutSubtreeIfNeeded()
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
            (row as? ActionRowHost)?.setHighlighted(index == selectedIndex)
        }
    }
}

@MainActor
private final class ActionRowHost: NSView {
    let rowIndex: Int
    private let highlightView = NSView()

    init(index: Int) {
        self.rowIndex = index
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 28).isActive = true

        highlightView.translatesAutoresizingMaskIntoConstraints = false
        highlightView.wantsLayer = true
        highlightView.layer?.cornerRadius = 6
        highlightView.layer?.backgroundColor = NSColor.clear.cgColor
        addSubview(highlightView)
        NSLayoutConstraint.activate([
            highlightView.topAnchor.constraint(equalTo: topAnchor),
            highlightView.leadingAnchor.constraint(equalTo: leadingAnchor),
            highlightView.trailingAnchor.constraint(equalTo: trailingAnchor),
            highlightView.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    func setHighlighted(_ highlighted: Bool) {
        highlightView.layer?.backgroundColor = highlighted
            ? NSColor.controlAccentColor.withAlphaComponent(0.08).cgColor
            : NSColor.clear.cgColor
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}
