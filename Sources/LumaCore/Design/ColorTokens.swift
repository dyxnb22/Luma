import AppKit

/// Centralized visual tokens for dashboard cards and launcher chrome.
@MainActor
public enum ColorTokens {
    public static var cardGradientTopAlpha: CGFloat {
        isDarkAppearance ? 0.50 : GeekStyleTokens.cardGradientTopAlphaLight
    }

    public static var cardGradientBottomAlpha: CGFloat {
        isDarkAppearance ? 0.55 : GeekStyleTokens.cardGradientBottomAlphaLight
    }

    public static let cardBorderAlpha: CGFloat = 0.32
    public static let cardBorderHighlightedAlpha: CGFloat = 0.85
    public static let cardTitleSubtitleAlpha: CGFloat = 0.62
    public static let cardStatusAlpha: CGFloat = 0.72
    public static let cardShortcutAlpha: CGFloat = 0.65
    public static let returnHintCapsuleAlpha: CGFloat = 0.24
    public static let listRowSelectionAlpha: CGFloat = LauncherChromeTokens.listRowSelectionAlpha
    public static let searchSurfaceAlpha: CGFloat = 0.42
    public static let searchSurfaceBorderAlpha: CGFloat = 0.35

    public static var panelBorderColor: NSColor {
        separatorBlend(alpha: LauncherChromeTokens.panelBorderAlpha)
    }

    public static var searchSurfaceFill: NSColor {
        controlFillBlend(alpha: searchSurfaceAlpha)
    }

    public static var searchSurfaceBorder: NSColor {
        separatorBlend(alpha: searchSurfaceBorderAlpha)
    }

    public static var listRowSelectionFill: NSColor {
        NSColor.controlAccentColor.withAlphaComponent(listRowSelectionAlpha)
    }

    public static var listRowHoverFill: NSColor {
        NSColor.labelColor.withAlphaComponent(LauncherChromeTokens.listRowHoverAlpha)
    }

    public static var performanceStripSurfaceFill: NSColor {
        NSColor.quaternaryLabelColor.withAlphaComponent(0.08)
    }

    public static var performanceStripNormalText: NSColor {
        .tertiaryLabelColor
    }

    public static var performanceStripMetricText: NSColor {
        .secondaryLabelColor
    }

    public static var performanceStripElevatedMetricText: NSColor {
        .labelColor.withAlphaComponent(0.78)
    }

    public static var performanceStripWarningAccent: NSColor {
        NSColor.systemOrange.withAlphaComponent(0.88)
    }

    private static var isDarkAppearance: Bool {
        NSApp.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
    }

    public static func cardIconTint(for topHex: String) -> NSColor {
        let base = color(hex: topHex)
        let blend: CGFloat = isDarkAppearance ? 0.35 : 0.25
        return base.blended(withFraction: blend, of: .white) ?? base
    }

    private static func separatorBlend(alpha: CGFloat) -> NSColor {
        NSColor.separatorColor.withAlphaComponent(alpha)
    }

    private static func controlFillBlend(alpha: CGFloat) -> NSColor {
        NSColor.controlBackgroundColor.withAlphaComponent(alpha)
    }

    public static func color(hex: String) -> NSColor {
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
