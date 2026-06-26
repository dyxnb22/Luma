import AppKit

/// Light-geek visual tokens: mono data, keycaps, accent strips, glass panels.
@MainActor
enum GeekStyleTokens {
    static let accentStripWidth: CGFloat = 4
    static let cardCornerRadius: CGFloat = 16
    static let panelCornerRadius: CGFloat = 12
    static let keycapCornerRadius: CGFloat = 6
    static let sidebarAccentWidth: CGFloat = 2

    static let cardTintAlpha: CGFloat = 0.14
    static let cardBorderAlpha: CGFloat = 0.45
    static let cardShadowOpacity: Float = 0.10
    static let cardShadowRadius: CGFloat = 10
    static let cardGradientTopAlphaLight: CGFloat = 0.58
    static let cardGradientBottomAlphaLight: CGFloat = 0.66
    static let cardHighlightGlowAlpha: CGFloat = 0.55
    static let glassPanelBorderAlpha: CGFloat = 0.38
    static let keycapBackgroundAlpha: CGFloat = 0.22
    static let keycapBorderAlpha: CGFloat = 0.48

    static func mono(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
        .monospacedSystemFont(ofSize: size, weight: weight)
    }

    static func monoDigit(size: CGFloat, weight: NSFont.Weight = .medium) -> NSFont {
        .monospacedDigitSystemFont(ofSize: size, weight: weight)
    }
}
