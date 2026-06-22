import Foundation

public struct TranslationOutcome: Sendable, Equatable {
    public let text: String
    public let detectedSourceLanguageCode: String?

    public init(text: String, detectedSourceLanguageCode: String? = nil) {
        self.text = text
        self.detectedSourceLanguageCode = detectedSourceLanguageCode
    }
}
