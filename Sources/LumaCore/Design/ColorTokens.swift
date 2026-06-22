import AppKit

/// Centralized visual tokens for dashboard cards and launcher chrome.
@MainActor
public enum ColorTokens {
    public static var cardGradientTopAlpha: CGFloat {
        isDarkAppearance ? 0.50 : 0.42
    }

    public static var cardGradientBottomAlpha: CGFloat {
        isDarkAppearance ? 0.55 : 0.48
    }

    public static let cardBorderAlpha: CGFloat = 0.32
    public static let cardBorderHighlightedAlpha: CGFloat = 0.85
    public static let cardTitleSubtitleAlpha: CGFloat = 0.62
    public static let cardStatusAlpha: CGFloat = 0.72
    public static let cardShortcutAlpha: CGFloat = 0.65
    public static let returnHintCapsuleAlpha: CGFloat = 0.22

    private static var isDarkAppearance: Bool {
        NSApp.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
    }

    public static func cardIconTint(for topHex: String) -> NSColor {
        let base = color(hex: topHex)
        let blend: CGFloat = isDarkAppearance ? 0.35 : 0.25
        return base.blended(withFraction: blend, of: .white) ?? base
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
