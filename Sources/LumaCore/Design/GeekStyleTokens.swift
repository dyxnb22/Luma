import AppKit

/// Light-geek visual tokens: mono data, keycaps, accent strips, glass panels.
@MainActor
public enum GeekStyleTokens {
    public static let accentStripWidth: CGFloat = 4
    public static let cardCornerRadius: CGFloat = 16
    public static let panelCornerRadius: CGFloat = 12
    public static let keycapCornerRadius: CGFloat = 6
    public static let sidebarAccentWidth: CGFloat = 2

    public static let cardTintAlpha: CGFloat = 0.14
    public static let cardBorderAlpha: CGFloat = 0.45
    public static let cardShadowOpacity: Float = 0.10
    public static let cardShadowRadius: CGFloat = 10
    public static let cardGradientTopAlphaLight: CGFloat = 0.58
    public static let cardGradientBottomAlphaLight: CGFloat = 0.66
    public static let cardHighlightGlowAlpha: CGFloat = 0.55
    public static let glassPanelBorderAlpha: CGFloat = 0.38
    public static let keycapBackgroundAlpha: CGFloat = 0.22
    public static let keycapBorderAlpha: CGFloat = 0.48

    public static func mono(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
        .monospacedSystemFont(ofSize: size, weight: weight)
    }

    public static func monoDigit(size: CGFloat, weight: NSFont.Weight = .medium) -> NSFont {
        .monospacedDigitSystemFont(ofSize: size, weight: weight)
    }
}
