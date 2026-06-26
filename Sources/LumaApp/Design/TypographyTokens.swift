import AppKit

@MainActor
enum TypographyTokens {
    static let caption2 = NSFont.systemFont(ofSize: 10)
    static let caption = NSFont.systemFont(ofSize: 12)
    static let body = NSFont.systemFont(ofSize: 13)
    static let callout = NSFont.systemFont(ofSize: 15)
    static let title3 = NSFont.systemFont(ofSize: 17, weight: .semibold)
    static let title2 = NSFont.systemFont(ofSize: 20, weight: .semibold)

    static func caption2(weight: NSFont.Weight = .regular) -> NSFont {
        .systemFont(ofSize: 10, weight: weight)
    }

    static func caption(weight: NSFont.Weight = .regular) -> NSFont {
        .systemFont(ofSize: 12, weight: weight)
    }

    static func monoCaption(weight: NSFont.Weight = .medium) -> NSFont {
        GeekStyleTokens.mono(size: 11, weight: weight)
    }

    static func monoMeta(weight: NSFont.Weight = .regular) -> NSFont {
        GeekStyleTokens.mono(size: 12, weight: weight)
    }
}
