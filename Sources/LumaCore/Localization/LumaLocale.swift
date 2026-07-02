import Foundation

/// User-facing language preference for Luma UI strings.
public enum LumaLocale {
    public static let preferenceKey = "luma.preferredLanguage"

    public enum Choice: String, CaseIterable, Sendable, Hashable {
        case system
        case en
        case zhHans = "zh-Hans"

        public var displayName: String {
            switch self {
            case .system: L10n.tr("settings.language.system")
            case .en: "English"
            case .zhHans: "简体中文"
            }
        }
    }

    public static var choice: Choice {
        get {
            guard let raw = UserDefaults.standard.string(forKey: preferenceKey),
                  let value = Choice(rawValue: raw) else { return .system }
            return value
        }
        set {
            UserDefaults.standard.set(newValue.rawValue, forKey: preferenceKey)
        }
    }

    public static var locale: Locale {
        switch choice {
        case .system: .autoupdatingCurrent
        case .en: Locale(identifier: "en")
        case .zhHans: Locale(identifier: "zh-Hans")
        }
    }
}
