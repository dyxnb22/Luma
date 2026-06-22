import AppKit
import LumaServices

@MainActor
final class SidebarAppRow: NSControl {
    enum Style {
        case app
        case window
    }

    private let onActivate: () -> Void
    private let iconView = NSImageView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let badgeHost = NSView()
    private let badgeLabel = NSTextField(labelWithString: "")

    init(
        app: NSRunningApplication,
        isActive: Bool,
        windowCount: Int? = nil,
        onActivate: @escaping () -> Void
    ) {
        self.onActivate = onActivate
        super.init(frame: .zero)
        configureChrome(style: .app, isHighlighted: isActive, indent: 0, height: 40)
        let icon = IconCache.shared.runningAppIcon(app)
        iconView.image = icon
        titleLabel.stringValue = app.localizedName ?? app.bundleIdentifier ?? "App"
        titleLabel.font = .systemFont(ofSize: 13, weight: .medium)
        if let windowCount, windowCount > 1 {
            setBadge("\(windowCount)")
        }
        setAccessibilityRole(.button)
        setAccessibilityLabel(app.localizedName ?? "Application")
        setAccessibilityHelp("Activates this application.")
    }

    init(
        window: OpenWindowSnapshot,
        icon: NSImage?,
        shortcutIndex: Int,
        onActivate: @escaping () -> Void
    ) {
        self.onActivate = onActivate
        super.init(frame: .zero)
        configureChrome(style: .window, isHighlighted: window.isFocused, indent: 32, height: 32)
        iconView.image = icon
        titleLabel.stringValue = window.title
        titleLabel.font = .systemFont(ofSize: 12)
        setBadge("\(shortcutIndex)")
        setAccessibilityRole(.button)
        setAccessibilityLabel(window.title)
        setAccessibilityHelp("Focuses this window.")
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func mouseDown(with event: NSEvent) {
        onActivate()
    }

    func setHighlighted(_ isHighlighted: Bool) {
        layer?.backgroundColor = isHighlighted
            ? NSColor.controlAccentColor.withAlphaComponent(0.22).cgColor
            : NSColor.clear.cgColor
    }

    @objc private func activate() {
        onActivate()
    }

    private func configureChrome(style: Style, isHighlighted: Bool, indent: CGFloat, height: CGFloat) {
        wantsLayer = true
        layer?.cornerRadius = 8
        layer?.cornerCurve = .continuous
        setHighlighted(isHighlighted)
        target = self
        action = #selector(activate)
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: height).isActive = true

        iconView.imageScaling = .scaleProportionallyDown
        iconView.imageAlignment = .alignLeft
        iconView.wantsLayer = true
        iconView.layer?.cornerRadius = style == .app ? 5.4 : 4
        iconView.layer?.cornerCurve = .continuous
        iconView.layer?.masksToBounds = true
        iconView.translatesAutoresizingMaskIntoConstraints = false

        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        badgeHost.wantsLayer = true
        badgeHost.layer?.cornerRadius = 8
        badgeHost.layer?.cornerCurve = .continuous
        badgeHost.layer?.backgroundColor = NSColor.secondaryLabelColor.withAlphaComponent(0.18).cgColor
        badgeHost.isHidden = true
        badgeHost.translatesAutoresizingMaskIntoConstraints = false

        badgeLabel.font = .monospacedDigitSystemFont(ofSize: 10, weight: .medium)
        badgeLabel.textColor = .secondaryLabelColor
        badgeLabel.alignment = .center
        badgeLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(iconView)
        addSubview(titleLabel)
        addSubview(badgeHost)
        badgeHost.addSubview(badgeLabel)

        let iconSize: CGFloat = style == .app ? 24 : 18

        NSLayoutConstraint.activate([
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: indent),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: iconSize),
            iconView.heightAnchor.constraint(equalToConstant: iconSize),

            badgeHost.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            badgeHost.centerYAnchor.constraint(equalTo: centerYAnchor),
            badgeHost.heightAnchor.constraint(equalToConstant: 18),
            badgeHost.widthAnchor.constraint(greaterThanOrEqualToConstant: 18),

            badgeLabel.topAnchor.constraint(equalTo: badgeHost.topAnchor, constant: 1),
            badgeLabel.bottomAnchor.constraint(equalTo: badgeHost.bottomAnchor, constant: -1),
            badgeLabel.leadingAnchor.constraint(equalTo: badgeHost.leadingAnchor, constant: 6),
            badgeLabel.trailingAnchor.constraint(equalTo: badgeHost.trailingAnchor, constant: -6),

            titleLabel.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 8),
            titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: badgeHost.leadingAnchor, constant: -6),
            titleLabel.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])
    }

    private func setBadge(_ text: String) {
        badgeLabel.stringValue = text
        badgeHost.isHidden = false
    }
}
