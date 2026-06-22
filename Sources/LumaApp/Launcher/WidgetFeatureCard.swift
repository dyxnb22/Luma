import AppKit
import LumaCore

@MainActor
final class WidgetFeatureCard: NSView {
    private let card: FeatureCard
    private let shortcutIndex: Int
    private let onSelect: (FeatureCard) -> Void
    private let glassView = NSVisualEffectView()
    private let gradientLayer = CAGradientLayer()
    private let highlightLayer = CAGradientLayer()
    private let iconView = NSImageView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(labelWithString: "")
    private let statusLabel = NSTextField(labelWithString: "")
    private let shortcutLabel = NSTextField(labelWithString: "")
    private var trackingArea: NSTrackingArea?

    init(
        card: FeatureCard,
        shortcutIndex: Int,
        statusSummary: String = "",
        onSelect: @escaping (FeatureCard) -> Void
    ) {
        self.card = card
        self.shortcutIndex = shortcutIndex
        self.onSelect = onSelect
        super.init(frame: .zero)
        wantsLayer = true
        layer?.cornerRadius = 24
        layer?.cornerCurve = .continuous
        layer?.masksToBounds = true
        layer?.borderWidth = 1
        layer?.borderColor = NSColor.white.withAlphaComponent(0.22).cgColor
        setupGlass()
        setupGradient()
        setupContent(statusSummary: statusSummary)
        setupTracking()
        setAccessibilityRole(.button)
        setAccessibilityLabel(card.title)
        setAccessibilityHelp("Opens \(card.title) module. Shortcut Command \(shortcutIndex).")
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func updateStatusSummary(_ summary: String) {
        statusLabel.stringValue = summary
    }

    override func layout() {
        super.layout()
        gradientLayer.frame = bounds
        highlightLayer.frame = CGRect(x: 0, y: bounds.height - 48, width: bounds.width, height: 48)
    }

    override func mouseDown(with event: NSEvent) {
        animateScale(to: 0.97, duration: 0.06)
    }

    override func mouseUp(with event: NSEvent) {
        animateScale(to: isMouseInBounds() ? 1.02 : 1.0, duration: 0.12)
        if isMouseInBounds() {
            onSelect(card)
        }
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }
        setupTracking()
    }

    override func mouseEntered(with event: NSEvent) {
        animateScale(to: 1.02, duration: 0.12)
    }

    override func mouseExited(with event: NSEvent) {
        animateScale(to: 1.0, duration: 0.12)
    }

    private func setupGlass() {
        glassView.material = .hudWindow
        glassView.blendingMode = .withinWindow
        glassView.state = .active
        glassView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(glassView)
        NSLayoutConstraint.activate([
            glassView.topAnchor.constraint(equalTo: topAnchor),
            glassView.leadingAnchor.constraint(equalTo: leadingAnchor),
            glassView.trailingAnchor.constraint(equalTo: trailingAnchor),
            glassView.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    private func setupGradient() {
        guard let style = card.widgetStyle else { return }
        gradientLayer.colors = [
            Self.color(hex: style.topHex).withAlphaComponent(0.88).cgColor,
            Self.color(hex: style.bottomHex).withAlphaComponent(0.92).cgColor
        ]
        gradientLayer.startPoint = CGPoint(x: 0.2, y: 1)
        gradientLayer.endPoint = CGPoint(x: 0.8, y: 0)
        gradientLayer.cornerRadius = 24
        layer?.insertSublayer(gradientLayer, at: 0)

        highlightLayer.colors = [
            NSColor.white.withAlphaComponent(0.28).cgColor,
            NSColor.white.withAlphaComponent(0.0).cgColor
        ]
        highlightLayer.startPoint = CGPoint(x: 0.5, y: 1)
        highlightLayer.endPoint = CGPoint(x: 0.5, y: 0.55)
        layer?.addSublayer(highlightLayer)
    }

    private func setupContent(statusSummary: String) {
        translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            heightAnchor.constraint(equalToConstant: 156)
        ])

        let symbolName = card.widgetStyle?.symbolName ?? "sparkles"
        let symbolConfig = NSImage.SymbolConfiguration(pointSize: 28, weight: .medium)
        iconView.image = NSImage(systemSymbolName: symbolName, accessibilityDescription: nil)?
            .withSymbolConfiguration(symbolConfig)
        iconView.contentTintColor = .white
        iconView.imageScaling = .scaleProportionallyDown
        iconView.translatesAutoresizingMaskIntoConstraints = false

        titleLabel.stringValue = card.title
        titleLabel.font = .systemFont(ofSize: 17, weight: .semibold)
        titleLabel.textColor = .white
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        subtitleLabel.stringValue = card.subtitle
        subtitleLabel.font = .systemFont(ofSize: 12, weight: .regular)
        subtitleLabel.textColor = NSColor.white.withAlphaComponent(0.82)
        subtitleLabel.lineBreakMode = .byTruncatingTail
        subtitleLabel.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.stringValue = statusSummary
        statusLabel.font = .systemFont(ofSize: 11, weight: .medium)
        statusLabel.textColor = NSColor.white.withAlphaComponent(0.72)
        statusLabel.lineBreakMode = .byTruncatingTail
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        shortcutLabel.stringValue = "⌘\(shortcutIndex)"
        shortcutLabel.font = .systemFont(ofSize: 11, weight: .semibold)
        shortcutLabel.textColor = NSColor.white.withAlphaComponent(0.65)
        shortcutLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(iconView)
        addSubview(titleLabel)
        addSubview(subtitleLabel)
        addSubview(statusLabel)
        addSubview(shortcutLabel)

        NSLayoutConstraint.activate([
            shortcutLabel.topAnchor.constraint(equalTo: topAnchor, constant: 14),
            shortcutLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -14),

            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 18),
            iconView.topAnchor.constraint(equalTo: topAnchor, constant: 18),
            iconView.widthAnchor.constraint(equalToConstant: 32),
            iconView.heightAnchor.constraint(equalToConstant: 32),

            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 18),
            titleLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -18),
            titleLabel.topAnchor.constraint(equalTo: iconView.bottomAnchor, constant: 14),

            subtitleLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            subtitleLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 3),

            statusLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            statusLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            statusLabel.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -16)
        ])
    }

    private func setupTracking() {
        let area = NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .activeInActiveApp, .inVisibleRect],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        trackingArea = area
    }

    private func animateScale(to scale: CGFloat, duration: TimeInterval) {
        CATransaction.begin()
        CATransaction.setAnimationDuration(duration)
        CATransaction.setAnimationTimingFunction(CAMediaTimingFunction(name: .easeOut))
        layer?.transform = CATransform3DMakeScale(scale, scale, 1)
        CATransaction.commit()
    }

    private func isMouseInBounds() -> Bool {
        guard let window else { return false }
        let location = window.mouseLocationOutsideOfEventStream
        let local = convert(location, from: nil)
        return bounds.contains(local)
    }

    private static func color(hex: String) -> NSColor {
        let sanitized = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        guard sanitized.count == 6, let value = UInt64(sanitized, radix: 16) else {
            return .gray
        }
        let red = CGFloat((value >> 16) & 0xFF) / 255
        let green = CGFloat((value >> 8) & 0xFF) / 255
        let blue = CGFloat(value & 0xFF) / 255
        return NSColor(red: red, green: green, blue: blue, alpha: 1)
    }
}
