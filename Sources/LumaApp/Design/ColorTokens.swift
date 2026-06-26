import AppKit
import LumaCore

/// Centralized visual tokens for module detail chrome and launcher list UI.
@MainActor
enum ColorTokens {
    static var cardGradientTopAlpha: CGFloat {
        isDarkAppearance ? 0.50 : GeekStyleTokens.cardGradientTopAlphaLight
    }

    static var cardGradientBottomAlpha: CGFloat {
        isDarkAppearance ? 0.55 : GeekStyleTokens.cardGradientBottomAlphaLight
    }

    static let cardBorderAlpha: CGFloat = 0.32
    static let cardBorderHighlightedAlpha: CGFloat = 0.85
    static let cardTitleSubtitleAlpha: CGFloat = 0.62
    static let cardStatusAlpha: CGFloat = 0.72
    static let cardShortcutAlpha: CGFloat = 0.65
    static let returnHintCapsuleAlpha: CGFloat = 0.24
    static let listRowSelectionAlpha: CGFloat = LauncherChromeTokens.listRowSelectionAlpha
    static let searchSurfaceAlpha: CGFloat = 0.42
    static let searchSurfaceBorderAlpha: CGFloat = 0.35

    static var panelBorderColor: NSColor {
        separatorBlend(alpha: LauncherChromeTokens.panelBorderAlpha)
    }

    static var searchSurfaceFill: NSColor {
        controlFillBlend(alpha: searchSurfaceAlpha)
    }

    static var searchSurfaceBorder: NSColor {
        separatorBlend(alpha: searchSurfaceBorderAlpha)
    }

    static var listRowSelectionFill: NSColor {
        NSColor.controlAccentColor.withAlphaComponent(listRowSelectionAlpha)
    }

    static var listRowHoverFill: NSColor {
        NSColor.labelColor.withAlphaComponent(LauncherChromeTokens.listRowHoverAlpha)
    }

    static var performanceStripSurfaceFill: NSColor {
        NSColor.quaternaryLabelColor.withAlphaComponent(0.08)
    }

    static var performanceStripNormalText: NSColor {
        .tertiaryLabelColor
    }

    static var performanceStripMetricText: NSColor {
        .secondaryLabelColor
    }

    static var performanceStripElevatedMetricText: NSColor {
        .labelColor.withAlphaComponent(0.78)
    }

    static var performanceStripWarningAccent: NSColor {
        NSColor.systemOrange.withAlphaComponent(0.88)
    }

    private static var isDarkAppearance: Bool {
        NSApp.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
    }

    static func cardIconTint(for topHex: String) -> NSColor {
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

    static func color(hex: String) -> NSColor {
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
