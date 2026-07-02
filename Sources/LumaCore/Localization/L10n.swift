import Foundation

/// Localized UI strings backed by `Resources/L10nStrings.json` (and `Localizable.xcstrings` for Xcode).
public enum L10n {
    private static let table: [String: [String: String]] = L10nTableLoader.load()

    public static func tr(_ key: String.LocalizationValue) -> String {
        tr(String(describing: key))
    }

    public static func tr(_ key: String.LocalizationValue, _ args: CVarArg...) -> String {
        tr(String(describing: key), args)
    }

    public static func tr(_ key: String) -> String {
        let language = languageCode
        if let value = table[key]?[language] ?? table[key]?["en"] {
            return value
        }
        return key
    }

    public static func tr(_ key: String, _ args: [CVarArg]) -> String {
        let format = tr(key)
        guard !args.isEmpty else { return format }
        return String(format: format, locale: LumaLocale.locale, arguments: args)
    }

    private static var languageCode: String {
        switch LumaLocale.choice {
        case .system:
            let preferred = Locale.autoupdatingCurrent.language.languageCode?.identifier ?? "en"
            return preferred.hasPrefix("zh") ? "zh-Hans" : "en"
        case .en: return "en"
        case .zhHans: return "zh-Hans"
        }
    }
}

enum L10nTableLoader {
    static func load() -> [String: [String: String]] {
        guard let url = Bundle.module.url(forResource: "L10nStrings", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let decoded = try? JSONDecoder().decode([String: [String: String]].self, from: data) else {
            return [:]
        }
        return decoded
    }
}
