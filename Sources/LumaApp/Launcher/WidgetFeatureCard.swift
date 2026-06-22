import AppKit
import LumaCore

@MainActor
final class WidgetFeatureCard: NSView {
    private let card: FeatureCard
    private let shortcutIndex: Int
    private let onSelect: (FeatureCard) -> Void
    private let gradientLayer = CAGradientLayer()
    private var trackingArea: NSTrackingArea?

    init(card: FeatureCard, shortcutIndex: Int, onSelect: @escaping (FeatureCard) -> Void) {
        self.card = card
        self.shortcutIndex = shortcutIndex
        self.onSelect = onSelect
        super.init(frame: .zero)
        wantsLayer = true
        layer?.cornerRadius = 27
        layer?.cornerCurve = .continuous
        layer?.masksToBounds = true
        setupGradient()
        setupContent()
        setupTracking()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layout() {
        super.layout()
        gradientLayer.frame = bounds
    }

    override func mouseDown(with event: NSEvent) {
        animateScale(to: 0.96, duration: 0.06)
    }

    override func mouseUp(with event: NSEvent) {
        animateScale(to: isMouseInBounds() ? 1.04 : 1.0, duration: 0.12)
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
        animateScale(to: 1.04, duration: 0.12)
    }

    override func mouseExited(with event: NSEvent) {
        animateScale(to: 1.0, duration: 0.12)
    }

    private func setupGradient() {
        guard let style = card.widgetStyle else { return }
        gradientLayer.colors = [
            Self.color(hex: style.topHex).cgColor,
            Self.color(hex: style.bottomHex).cgColor
        ]
        gradientLayer.startPoint = CGPoint(x: 0.5, y: 0)
        gradientLayer.endPoint = CGPoint(x: 0.5, y: 1)
        layer?.insertSublayer(gradientLayer, at: 0)
    }

    private func setupContent() {
        translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            widthAnchor.constraint(equalToConstant: 120),
            heightAnchor.constraint(equalToConstant: 120)
        ])

        let symbolName = card.widgetStyle?.symbolName ?? "sparkles"
        let symbolConfig = NSImage.SymbolConfiguration(pointSize: 48, weight: .regular)
        let iconView = NSImageView()
        iconView.image = NSImage(systemSymbolName: symbolName, accessibilityDescription: nil)?
            .withSymbolConfiguration(symbolConfig)
        iconView.contentTintColor = .white
        iconView.imageScaling = .scaleProportionallyDown
        iconView.translatesAutoresizingMaskIntoConstraints = false

        let title = NSTextField(labelWithString: card.title)
        title.font = .systemFont(ofSize: 13, weight: .semibold)
        title.textColor = .white
        title.alignment = .center
        title.lineBreakMode = .byTruncatingTail
        title.translatesAutoresizingMaskIntoConstraints = false

        let shortcut = NSTextField(labelWithString: "⌘\(shortcutIndex)")
        shortcut.font = .systemFont(ofSize: 10, weight: .medium)
        shortcut.textColor = NSColor.white.withAlphaComponent(0.7)
        shortcut.translatesAutoresizingMaskIntoConstraints = false

        addSubview(iconView)
        addSubview(title)
        addSubview(shortcut)

        NSLayoutConstraint.activate([
            shortcut.topAnchor.constraint(equalTo: topAnchor, constant: 10),
            shortcut.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
            iconView.centerXAnchor.constraint(equalTo: centerXAnchor),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor, constant: -8),
            iconView.widthAnchor.constraint(equalToConstant: 48),
            iconView.heightAnchor.constraint(equalToConstant: 48),
            title.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            title.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            title.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -14)
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
        NSAnimationContext.runAnimationGroup { context in
            context.duration = duration
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            layer?.transform = CATransform3DMakeScale(scale, scale, 1)
        }
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
