import Foundation
import NaturalLanguage

enum TranslationLanguageDetector {
    static func detectedLanguageCode(for text: String) -> String? {
        let recognizer = NLLanguageRecognizer()
        recognizer.processString(text)
        guard let dominant = recognizer.dominantLanguage else { return nil }
        return languageCodeString(from: dominant)
    }

    static func detectSourceLanguage(for text: String) -> Locale.Language? {
        guard let code = detectedLanguageCode(for: text) else { return nil }
        return Locale.Language(identifier: code)
    }

    static func languageCodeString(from language: NLLanguage) -> String {
        switch language {
        case .simplifiedChinese:
            return "zh-Hans"
        case .traditionalChinese:
            return "zh-Hant"
        default:
            return language.rawValue
        }
    }
}
