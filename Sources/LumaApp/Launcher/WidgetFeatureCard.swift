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
    private var isHighlighted = false

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
        layer?.borderColor = NSColor.white.withAlphaComponent(ColorTokens.cardBorderAlpha).cgColor
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
        statusLabel.isHidden = summary.isEmpty
    }

    func setHighlighted(_ on: Bool) {
        guard isHighlighted != on else { return }
        isHighlighted = on
        let borderAlpha = on ? ColorTokens.cardBorderHighlightedAlpha : ColorTokens.cardBorderAlpha
        NSAnimationContext.runAnimationGroup { context in
            context.duration = on ? 0.06 : 0.12
            layer?.borderColor = NSColor.white.withAlphaComponent(borderAlpha).cgColor
            layer?.borderWidth = on ? 2 : 1
        }
    }

    override var intrinsicContentSize: NSSize {
        NSSize(width: 220, height: 132)
    }

    override func layout() {
        super.layout()
        gradientLayer.frame = bounds
        highlightLayer.frame = CGRect(x: 0, y: bounds.height - 40, width: bounds.width, height: 40)
    }

    override func mouseDown(with event: NSEvent) {
        animateScale(to: 0.97, duration: MotionTokens.scaleInDuration)
    }

    override func mouseUp(with event: NSEvent) {
        animateScale(to: isMouseInBounds() ? 1.02 : 1.0, duration: MotionTokens.scaleOutDuration)
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
        animateScale(to: 1.02, duration: MotionTokens.scaleOutDuration)
    }

    override func mouseExited(with event: NSEvent) {
        animateScale(to: 1.0, duration: MotionTokens.scaleOutDuration)
    }

    private func setupGlass() {
        glassView.material = .popover
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
        let top = ColorTokens.color(hex: style.topHex)
        let bottom = ColorTokens.color(hex: style.bottomHex)
        gradientLayer.colors = [
            top.withAlphaComponent(ColorTokens.cardGradientTopAlpha).cgColor,
            bottom.withAlphaComponent(ColorTokens.cardGradientBottomAlpha).cgColor
        ]
        gradientLayer.startPoint = CGPoint(x: 0.2, y: 1)
        gradientLayer.endPoint = CGPoint(x: 0.8, y: 0)
        gradientLayer.cornerRadius = 24
        layer?.insertSublayer(gradientLayer, at: 0)

        highlightLayer.colors = [
            NSColor.white.withAlphaComponent(0.42).cgColor,
            NSColor.white.withAlphaComponent(0.12).cgColor,
            NSColor.white.withAlphaComponent(0.0).cgColor
        ]
        highlightLayer.locations = [0.0, 0.35, 1.0]
        highlightLayer.startPoint = CGPoint(x: 0.5, y: 1)
        highlightLayer.endPoint = CGPoint(x: 0.5, y: 0.55)
        layer?.addSublayer(highlightLayer)
    }

    private func setupContent(statusSummary: String) {
        translatesAutoresizingMaskIntoConstraints = false

        let symbolName = card.widgetStyle?.symbolName ?? "sparkles"
        let topHex = card.widgetStyle?.topHex ?? "#888888"
        let symbolConfig = NSImage.SymbolConfiguration(pointSize: 28, weight: .medium)
        iconView.image = NSImage(systemSymbolName: symbolName, accessibilityDescription: nil)?
            .withSymbolConfiguration(symbolConfig)
        iconView.contentTintColor = ColorTokens.cardIconTint(for: topHex)
        iconView.imageScaling = .scaleProportionallyDown
        iconView.translatesAutoresizingMaskIntoConstraints = false

        titleLabel.stringValue = card.title
        titleLabel.font = .systemFont(ofSize: 17, weight: .semibold)
        titleLabel.textColor = .white
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        subtitleLabel.stringValue = card.subtitle
        subtitleLabel.font = .systemFont(ofSize: 12, weight: .regular)
        subtitleLabel.textColor = NSColor.white.withAlphaComponent(ColorTokens.cardTitleSubtitleAlpha)
        subtitleLabel.lineBreakMode = .byTruncatingTail
        subtitleLabel.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.stringValue = statusSummary
        statusLabel.font = .systemFont(ofSize: 11, weight: .medium)
        statusLabel.textColor = NSColor.white.withAlphaComponent(ColorTokens.cardStatusAlpha)
        statusLabel.lineBreakMode = .byTruncatingTail
        statusLabel.isHidden = statusSummary.isEmpty
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        shortcutLabel.stringValue = "⌘\(shortcutIndex)"
        shortcutLabel.font = .systemFont(ofSize: 11, weight: .semibold)
        shortcutLabel.textColor = NSColor.white.withAlphaComponent(ColorTokens.cardShortcutAlpha)
        shortcutLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(iconView)
        addSubview(titleLabel)
        addSubview(subtitleLabel)
        addSubview(statusLabel)
        addSubview(shortcutLabel)

        NSLayoutConstraint.activate([
            shortcutLabel.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            shortcutLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -14),

            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            iconView.topAnchor.constraint(equalTo: topAnchor, constant: 14),
            iconView.widthAnchor.constraint(equalToConstant: 28),
            iconView.heightAnchor.constraint(equalToConstant: 28),

            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            titleLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16),
            titleLabel.topAnchor.constraint(equalTo: iconView.bottomAnchor, constant: 10),

            subtitleLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            subtitleLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 2),

            statusLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            statusLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            statusLabel.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12)
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
        guard !MotionTokens.shouldReduceMotion else {
            layer?.transform = scale == 1.0 ? CATransform3DIdentity : CATransform3DMakeScale(scale, scale, 1)
            return
        }
        CATransaction.begin()
        CATransaction.setAnimationDuration(duration)
        CATransaction.setAnimationTimingFunction(CAMediaTimingFunction(name: .easeOut))
        layer?.transform = scale == 1.0 ? CATransform3DIdentity : CATransform3DMakeScale(scale, scale, 1)
        CATransaction.commit()
    }

    private func isMouseInBounds() -> Bool {
        guard let window else { return false }
        let location = window.mouseLocationOutsideOfEventStream
        let local = convert(location, from: nil)
        return bounds.contains(local)
    }
}

enum MotionTokens {
    static var shouldReduceMotion: Bool {
        NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
    }

    static var scaleInDuration: TimeInterval { shouldReduceMotion ? 0.03 : 0.06 }
    static var scaleOutDuration: TimeInterval { shouldReduceMotion ? 0.06 : 0.12 }
    static var panelShowDuration: TimeInterval { shouldReduceMotion ? 0.08 : 0.14 }
    static var panelHideDuration: TimeInterval { shouldReduceMotion ? 0.05 : 0.10 }
}
