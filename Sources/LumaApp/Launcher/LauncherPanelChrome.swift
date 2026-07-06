@preconcurrency import AppKit
import LumaCore

/// Light glass chrome for the launcher panel — native frosted surface, not heavy liquid glass.
@MainActor
enum LauncherPanelChrome {
    private static let overlayIdentifier = "launcherChromeOverlay"

    static func install(on view: NSView, glassBackground: NSVisualEffectView) {
        // Do not set wantsLayer on the root content view — default layer anchorPoint (0.5, 0.5)
        // shifts painted content horizontally when subviews relayout (e.g. after typing `clip`).
        view.clipsToBounds = true

        glassBackground.material = .popover
        glassBackground.blendingMode = .behindWindow
        glassBackground.state = .active
        glassBackground.translatesAutoresizingMaskIntoConstraints = false
        glassBackground.wantsLayer = true
        glassBackground.layer?.cornerRadius = LauncherChromeTokens.panelCornerRadius
        glassBackground.layer?.cornerCurve = .continuous
        glassBackground.layer?.masksToBounds = true
        view.addSubview(glassBackground, positioned: .below, relativeTo: nil)

        let overlay = NSView()
        overlay.identifier = NSUserInterfaceItemIdentifier(overlayIdentifier)
        overlay.translatesAutoresizingMaskIntoConstraints = false
        overlay.wantsLayer = true
        overlay.layer?.cornerRadius = LauncherChromeTokens.panelCornerRadius
        overlay.layer?.cornerCurve = .continuous
        overlay.layer?.masksToBounds = true
        overlay.layer?.backgroundColor = NSColor.clear.cgColor
        overlay.layer?.borderWidth = LauncherChromeTokens.panelBorderWidth
        overlay.layer?.borderColor = ColorTokens.panelBorderColor.cgColor
        view.addSubview(overlay, positioned: .above, relativeTo: glassBackground)

        NSLayoutConstraint.activate([
            glassBackground.topAnchor.constraint(equalTo: view.topAnchor),
            glassBackground.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            glassBackground.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            glassBackground.bottomAnchor.constraint(equalTo: view.bottomAnchor),

            overlay.topAnchor.constraint(equalTo: view.topAnchor),
            overlay.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            overlay.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            overlay.bottomAnchor.constraint(equalTo: view.bottomAnchor)
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
        overlay.layer?.addSublayer(sheen)

        let depth = CAGradientLayer()
        depth.colors = [
            NSColor.clear.cgColor,
            NSColor.black.withAlphaComponent(LauncherChromeTokens.panelDepthBottomAlpha).cgColor
        ]
        depth.locations = [0.55, 1.0]
        depth.startPoint = CGPoint(x: 0.5, y: 1.0)
        depth.endPoint = CGPoint(x: 0.5, y: 0.0)
        depth.name = "launcherDepth"
        overlay.layer?.addSublayer(depth)
    }

  nonisolated static func layoutChromeLayers(on view: NSView) {
        guard let overlay = chromeOverlay(in: view) else { return }
        let bounds = overlay.bounds
        overlay.layer?.sublayers?
            .first { $0.name == "launcherSheen" }?
            .frame = bounds
        overlay.layer?.sublayers?
            .first { $0.name == "launcherDepth" }?
            .frame = bounds
    }

    nonisolated static func layoutSheen(on view: NSView) {
        layoutChromeLayers(on: view)
    }

    nonisolated private static func chromeOverlay(in view: NSView) -> NSView? {
        view.subviews.first { $0.identifier?.rawValue == "launcherChromeOverlay" }
    }
}
