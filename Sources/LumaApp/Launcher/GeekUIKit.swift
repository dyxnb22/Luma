import AppKit
import LumaCore

/// Shared light-geek AppKit styling: glass panels, keycaps, action buttons, accent strips.
@MainActor
enum GeekUIKit {
    static func installAccentStrip(on view: NSView, color: NSColor) -> CALayer {
        let strip = CALayer()
        strip.name = "geekAccentStrip"
        strip.backgroundColor = color.cgColor
        strip.cornerRadius = 1.5
        strip.cornerCurve = .continuous
        view.layer?.addSublayer(strip)
        return strip
    }

    static func layoutAccentStrip(_ strip: CALayer, in bounds: CGRect) {
        strip.frame = CGRect(
            x: 0,
            y: 0,
            width: GeekStyleTokens.accentStripWidth,
            height: bounds.height
        )
    }

    static func installSidebarAccent(on row: NSView) -> CALayer {
        row.wantsLayer = true
        let strip = CALayer()
        strip.name = "geekSidebarAccent"
        strip.backgroundColor = NSColor.controlAccentColor.cgColor
        strip.cornerRadius = 1
        strip.cornerCurve = .continuous
        strip.isHidden = true
        row.layer?.addSublayer(strip)
        return strip
    }

    static func layoutSidebarAccent(_ strip: CALayer, in bounds: CGRect) {
        strip.frame = CGRect(
            x: 2,
            y: 6,
            width: GeekStyleTokens.sidebarAccentWidth,
            height: max(0, bounds.height - 12)
        )
    }

    static func configureGlassPanel(_ panel: NSView, accentHex: String?) {
        panel.wantsLayer = true
        panel.layer?.cornerRadius = GeekStyleTokens.panelCornerRadius
        panel.layer?.cornerCurve = .continuous
        panel.layer?.borderWidth = 0.5
        panel.layer?.borderColor = NSColor.white.withAlphaComponent(GeekStyleTokens.glassPanelBorderAlpha).cgColor
        panel.layer?.backgroundColor = NSColor.clear.cgColor
        panel.layer?.masksToBounds = true

        if panel.subviews.contains(where: { ($0 as? NSVisualEffectView)?.identifier?.rawValue == "geekGlass" }) {
            return
        }

        let glass = NSVisualEffectView()
        glass.identifier = NSUserInterfaceItemIdentifier("geekGlass")
        glass.material = .contentBackground
        glass.blendingMode = .withinWindow
        glass.state = .active
        glass.translatesAutoresizingMaskIntoConstraints = false
        panel.addSubview(glass, positioned: .below, relativeTo: nil)
        NSLayoutConstraint.activate([
            glass.topAnchor.constraint(equalTo: panel.topAnchor),
            glass.leadingAnchor.constraint(equalTo: panel.leadingAnchor),
            glass.trailingAnchor.constraint(equalTo: panel.trailingAnchor),
            glass.bottomAnchor.constraint(equalTo: panel.bottomAnchor)
        ])

        if let accentHex {
            _ = installAccentStrip(on: panel, color: ColorTokens.color(hex: accentHex))
        }
    }

    static func layoutPanelAccentStrip(on panel: NSView) {
        guard let strip = panel.layer?.sublayers?.first(where: { $0.name == "geekAccentStrip" }) else { return }
        layoutAccentStrip(strip, in: panel.bounds)
    }

    static func configureLightKeycap(host: NSView, label: NSTextField, text: String) {
        host.wantsLayer = true
        host.layer?.cornerRadius = GeekStyleTokens.keycapCornerRadius
        host.layer?.cornerCurve = .continuous
        host.layer?.backgroundColor = NSColor.quaternaryLabelColor.withAlphaComponent(0.55).cgColor
        host.layer?.borderWidth = 0.5
        host.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.65).cgColor

        label.stringValue = text
        label.font = GeekStyleTokens.mono(size: 10, weight: .semibold)
        label.textColor = .secondaryLabelColor
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.alignment = .center
    }

    static func configureKeycap(host: NSView, label: NSTextField, text: String) {
        host.wantsLayer = true
        host.layer?.cornerRadius = GeekStyleTokens.keycapCornerRadius
        host.layer?.cornerCurve = .continuous
        host.layer?.backgroundColor = NSColor.white.withAlphaComponent(GeekStyleTokens.keycapBackgroundAlpha).cgColor
        host.layer?.borderWidth = 0.5
        host.layer?.borderColor = NSColor.white.withAlphaComponent(GeekStyleTokens.keycapBorderAlpha).cgColor

        label.stringValue = text
        label.font = GeekStyleTokens.mono(size: 10, weight: .semibold)
        label.textColor = NSColor.white.withAlphaComponent(0.88)
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.alignment = .center
    }

    static func stylePrimaryButton(_ button: NSButton) {
        button.bezelStyle = .rounded
        button.font = .systemFont(ofSize: 13, weight: .semibold)
        button.contentTintColor = .white
        button.bezelColor = .controlAccentColor
    }

    static func styleSecondaryButton(_ button: NSButton) {
        button.bezelStyle = .rounded
        button.font = .systemFont(ofSize: 12, weight: .medium)
        button.contentTintColor = .labelColor
        button.bezelColor = NSColor.secondaryLabelColor.withAlphaComponent(0.12)
    }

    static func styleLanguageChip(_ chip: NSButton, selected: Bool) {
        chip.bezelStyle = .rounded
        chip.font = .systemFont(ofSize: 11, weight: .medium)
        if selected {
            chip.bezelColor = .controlAccentColor
            chip.contentTintColor = .white
            chip.alphaValue = 1
        } else {
            chip.bezelColor = NSColor.secondaryLabelColor.withAlphaComponent(0.14)
            chip.contentTintColor = .labelColor
            chip.alphaValue = 0.9
        }
    }
}

extension NSView {
    func geekLayoutAccentLayers() {
        guard let sublayers = layer?.sublayers else { return }
        for sublayer in sublayers where sublayer.name == "geekAccentStrip" {
            GeekUIKit.layoutAccentStrip(sublayer, in: bounds)
        }
        for sublayer in sublayers where sublayer.name == "geekSidebarAccent" {
            GeekUIKit.layoutSidebarAccent(sublayer, in: bounds)
        }
    }
}

@MainActor
final class GeekGlassPanel: NSView {
    init(accentHex: String?) {
        super.init(frame: .zero)
        GeekUIKit.configureGlassPanel(self, accentHex: accentHex)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layout() {
        super.layout()
        geekLayoutAccentLayers()
    }
}
