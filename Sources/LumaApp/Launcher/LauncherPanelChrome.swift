import AppKit
import LumaCore

/// Light glass chrome for the launcher panel — native frosted surface, not heavy liquid glass.
@MainActor
enum LauncherPanelChrome {
    static func install(on view: NSView, glassBackground: NSVisualEffectView) {
        view.wantsLayer = true
        view.layer?.cornerRadius = LauncherChromeTokens.panelCornerRadius
        view.layer?.cornerCurve = .continuous
        view.layer?.masksToBounds = true
        view.layer?.borderWidth = LauncherChromeTokens.panelBorderWidth
        view.layer?.borderColor = ColorTokens.panelBorderColor.cgColor

        // Popover material reads closer to Spotlight's frosted center panel.
        glassBackground.material = .popover
        glassBackground.blendingMode = .behindWindow
        glassBackground.state = .active
        glassBackground.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(glassBackground, positioned: .below, relativeTo: nil)

        NSLayoutConstraint.activate([
            glassBackground.topAnchor.constraint(equalTo: view.topAnchor),
            glassBackground.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            glassBackground.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            glassBackground.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])

        let sheen = CAGradientLayer()
        sheen.colors = [
            NSColor.white.withAlphaComponent(LauncherChromeTokens.panelSheenTopAlpha).cgColor,
            NSColor.white.withAlphaComponent(LauncherChromeTokens.panelSheenMidAlpha).cgColor,
            NSColor.clear.cgColor
        ]
        sheen.locations = [0.0, 0.35, 1.0]
        sheen.startPoint = CGPoint(x: 0.5, y: 1.0)
        sheen.endPoint = CGPoint(x: 0.5, y: 0.0)
        sheen.name = "launcherSheen"
        view.layer?.addSublayer(sheen)
    }

    static func layoutSheen(on view: NSView) {
        view.layer?.sublayers?
            .first { $0.name == "launcherSheen" }?
            .frame = view.bounds
    }
}
