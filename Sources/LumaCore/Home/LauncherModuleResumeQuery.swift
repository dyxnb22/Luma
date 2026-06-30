import Foundation

/// Normalizes launcher queries when resuming module search flows.
public enum LauncherModuleResumeQuery {
    private static let notesID = ModuleIdentifier(rawValue: "luma.notes")
    private static let wordbookID = ModuleIdentifier(rawValue: "luma.wordbook")
    private static let mediaID = ModuleIdentifier(rawValue: "luma.media")
    private static let projectsID = ModuleIdentifier(rawValue: "luma.projects")
    private static let secretsID = ModuleIdentifier(rawValue: "luma.secrets")

    public static let roundTripModules: Set<ModuleIdentifier> = [
        notesID, wordbookID, mediaID, projectsID, secretsID
    ]

    public static func normalizedQuery(for module: ModuleIdentifier, raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty { return raw }
        switch module {
        case notesID: return "n "
        case wordbookID: return "word "
        case mediaID: return "rec "
        case projectsID: return "proj "
        case secretsID: return "sec "
        default: return raw
        }
    }

    public static func resumeTitle(for module: ModuleIdentifier) -> String {
        switch module {
        case notesID: return "Resume Notes search"
        case wordbookID: return "Resume Wordbook"
        case mediaID: return "Resume Records"
        case projectsID: return "Resume Projects search"
        case secretsID: return "Resume Secrets search"
        default: return "Resume last search"
        }
    }
}
