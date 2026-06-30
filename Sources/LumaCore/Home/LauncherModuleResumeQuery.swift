import Foundation

/// Normalizes launcher queries when resuming Notes, Wordbook, or Records flows.
public enum LauncherModuleResumeQuery {
    private static let notesID = ModuleIdentifier(rawValue: "luma.notes")
    private static let wordbookID = ModuleIdentifier(rawValue: "luma.wordbook")
    private static let mediaID = ModuleIdentifier(rawValue: "luma.media")

    public static let roundTripModules: Set<ModuleIdentifier> = [notesID, wordbookID, mediaID]

    public static func normalizedQuery(for module: ModuleIdentifier, raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty { return raw }
        switch module {
        case notesID: return "n "
        case wordbookID: return "word "
        case mediaID: return "rec "
        default: return raw
        }
    }

    public static func resumeTitle(for module: ModuleIdentifier) -> String {
        switch module {
        case notesID: return "Resume Notes search"
        case wordbookID: return "Resume Wordbook"
        case mediaID: return "Resume Records"
        default: return "Resume last search"
        }
    }
}
