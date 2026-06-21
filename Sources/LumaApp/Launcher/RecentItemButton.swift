import AppKit

@MainActor
final class RecentItemButton: NSButton {
    private let item: RecentDashboardItem

    init(item: RecentDashboardItem) {
        self.item = item
        super.init(frame: .zero)
        title = item.title
        image = item.icon
        imagePosition = .imageAbove
        alignment = .center
        font = .systemFont(ofSize: 11, weight: .medium)
        isBordered = false
        wantsLayer = true
        layer?.cornerRadius = 16
        layer?.cornerCurve = .continuous
        layer?.backgroundColor = NSColor.controlBackgroundColor.withAlphaComponent(0.34).cgColor
        target = self
        action = #selector(open)

        NSLayoutConstraint.activate([
            widthAnchor.constraint(equalToConstant: 92),
            heightAnchor.constraint(equalToConstant: 86)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    @objc private func open() {
        item.open()
    }
}
