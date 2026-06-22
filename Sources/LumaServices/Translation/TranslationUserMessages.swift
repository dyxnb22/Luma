import Foundation

public enum TranslationUserMessages {
    public static func message(for error: Error) -> String {
        if let err = error as? SystemTranslationError {
            switch err {
            case .shortcutUnavailable:
                return "Create a Shortcut named \"Luma Translate\" or allow Apple Translation to download language packs."
            case .shortcutTimedOut:
                return "Shortcut translation timed out. Check the \"Luma Translate\" shortcut or use Apple Translation."
            case .emptyOutput:
                return "Translation returned empty output. Try again after language models finish downloading."
            case .frameworkUnavailable:
                return "Apple Translation is unavailable on this Mac. Create a Shortcut named \"Luma Translate\"."
            case .languagePackRequired:
                return "Download the required language pack when prompted, or open System Settings → General → Language & Region."
            }
        }

        let nsError = error as NSError
        if nsError.domain.contains("Translation") || nsError.domain.contains("translation") {
            return "Download the required language pack when prompted, or open System Settings → General → Language & Region."
        }

        return "Translation failed. Allow language downloads when prompted, or create a Shortcut named \"Luma Translate\"."
    }

    public static func shouldTranslate(_ text: String) -> Bool {
        !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}
