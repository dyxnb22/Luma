import AppKit
import LumaCore

@MainActor
final class WidgetResultRow: NSControl {
    private let item: ResultItem
    private let onRun: (ResultItem) -> Void
    private let iconView = NSImageView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(labelWithString: "")
    private let returnHint = NSTextField(labelWithString: "↩")

    init(item: ResultItem, isSelected: Bool, onRun: @escaping (ResultItem) -> Void) {
        self.item = item
        self.onRun = onRun
        super.init(frame: .zero)
        wantsLayer = true
        layer?.cornerRadius = 12
        layer?.cornerCurve = .continuous
        setSelected(isSelected)
        setup()
        target = self
        action = #selector(run)
        setAccessibilityRole(.button)
        setAccessibilityLabel(item.title)
        if let subtitle = item.subtitle, !subtitle.isEmpty {
            setAccessibilityHelp(subtitle)
        }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func mouseDown(with event: NSEvent) {
        onRun(item)
    }

    func setSelected(_ isSelected: Bool) {
        layer?.backgroundColor = isSelected
            ? NSColor.controlAccentColor.withAlphaComponent(0.22).cgColor
            : NSColor.clear.cgColor
        returnHint.isHidden = !isSelected
    }

    @objc private func run() {
        onRun(item)
    }

    private func setup() {
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 56).isActive = true

        iconView.image = Self.iconImage(for: item.icon)
        iconView.imageScaling = .scaleProportionallyDown
        iconView.wantsLayer = true
        iconView.layer?.cornerRadius = 8
        iconView.layer?.cornerCurve = .continuous
        iconView.layer?.masksToBounds = true
        iconView.translatesAutoresizingMaskIntoConstraints = false

        titleLabel.stringValue = item.title
        titleLabel.font = .systemFont(ofSize: 15, weight: .medium)
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        subtitleLabel.stringValue = item.subtitle ?? ""
        subtitleLabel.font = .systemFont(ofSize: 12)
        subtitleLabel.textColor = .secondaryLabelColor
        subtitleLabel.lineBreakMode = .byTruncatingTail
        subtitleLabel.translatesAutoresizingMaskIntoConstraints = false

        returnHint.font = .systemFont(ofSize: 13, weight: .medium)
        returnHint.textColor = .tertiaryLabelColor
        returnHint.isHidden = true
        returnHint.translatesAutoresizingMaskIntoConstraints = false

        addSubview(iconView)
        addSubview(titleLabel)
        addSubview(subtitleLabel)
        addSubview(returnHint)

        NSLayoutConstraint.activate([
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 14),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: 36),
            iconView.heightAnchor.constraint(equalToConstant: 36),
            titleLabel.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 12),
            titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: returnHint.leadingAnchor, constant: -8),
            titleLabel.topAnchor.constraint(equalTo: topAnchor, constant: 10),
            subtitleLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            subtitleLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 2),
            returnHint.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -14),
            returnHint.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])
    }

    private static func iconImage(for icon: LumaCore.IconRef) -> NSImage? {
        switch icon {
        case .bundleID(let bundleID):
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) {
                return IconCache.shared.appIcon(for: url)
            }
            return NSImage(systemSymbolName: "app.fill", accessibilityDescription: nil)
        case .symbol(let symbol):
            return NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        case .file(let url):
            return IconCache.shared.appIcon(for: url)
        case .none:
            return NSImage(systemSymbolName: "sparkles", accessibilityDescription: nil)
        }
    }
}
