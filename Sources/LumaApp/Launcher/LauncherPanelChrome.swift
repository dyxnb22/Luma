import AppKit

/// Liquid-glass chrome for the launcher panel (single merged gradient layer).
@MainActor
enum LauncherPanelChrome {
    static func install(on view: NSView, glassBackground: NSVisualEffectView) {
        view.wantsLayer = true
        view.layer?.cornerRadius = 20
        view.layer?.cornerCurve = .continuous
        view.layer?.masksToBounds = true
        view.layer?.borderWidth = 1
        view.layer?.borderColor = NSColor.white.withAlphaComponent(0.18).cgColor

        glassBackground.material = .underWindowBackground
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
            NSColor.white.withAlphaComponent(0.24).cgColor,
            NSColor.white.withAlphaComponent(0.06).cgColor,
            NSColor(calibratedRed: 0.80, green: 0.92, blue: 1.0, alpha: 0.20).cgColor,
            NSColor(calibratedRed: 0.96, green: 0.93, blue: 1.0, alpha: 0.12).cgColor,
            NSColor.white.withAlphaComponent(0.10).cgColor
        ]
        sheen.locations = [0.0, 0.22, 0.48, 0.78, 1.0]
        sheen.startPoint = CGPoint(x: 0.0, y: 1.0)
        sheen.endPoint = CGPoint(x: 1.0, y: 0.0)
        sheen.name = "launcherSheen"
        view.layer?.addSublayer(sheen)
    }

    static func layoutSheen(on view: NSView) {
        view.layer?.sublayers?
            .first { $0.name == "launcherSheen" }?
            .frame = view.bounds
    }
}
