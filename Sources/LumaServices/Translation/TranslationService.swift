import Foundation
import LumaCore
import LumaInfrastructure

public actor TranslationService: TranslationClient {
    private let config: ConfigurationStore

    public init(config: ConfigurationStore = ConfigurationStore()) {
        self.config = config
    }

    public func translate(_ text: String) async throws -> TranslationOutcome {
        let target = await config.translationTargetLanguage()
        return try await SystemTranslationService().translate(text, targetLanguageIdentifier: target)
    }
}
