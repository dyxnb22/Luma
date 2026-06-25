import AppKit

@MainActor
public enum TypographyTokens {
    public static let caption2 = NSFont.systemFont(ofSize: 10)
    public static let caption = NSFont.systemFont(ofSize: 12)
    public static let body = NSFont.systemFont(ofSize: 13)
    public static let callout = NSFont.systemFont(ofSize: 15)
    public static let title3 = NSFont.systemFont(ofSize: 17, weight: .semibold)
    public static let title2 = NSFont.systemFont(ofSize: 20, weight: .semibold)

    public static func caption2(weight: NSFont.Weight = .regular) -> NSFont {
        .systemFont(ofSize: 10, weight: weight)
    }

    public static func caption(weight: NSFont.Weight = .regular) -> NSFont {
        .systemFont(ofSize: 12, weight: weight)
    }

    public static func monoCaption(weight: NSFont.Weight = .medium) -> NSFont {
        GeekStyleTokens.mono(size: 11, weight: weight)
    }

    public static func monoMeta(weight: NSFont.Weight = .regular) -> NSFont {
        GeekStyleTokens.mono(size: 12, weight: weight)
    }
}
