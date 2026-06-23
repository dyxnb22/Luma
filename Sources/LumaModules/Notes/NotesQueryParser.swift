import Foundation

public enum NotesQuery: Equatable, Sendable {
    case help
    case listRecents
    case search(String)
    case metaSearch(NotesMetaFilter)
    case new(title: String, template: String?)
    case daily
    case reviewWeek
    case doctor
}

public enum NotesQueryParser {
    /// Returns payload after the module trigger, or nil if the query does not target Notes.
    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()

        if lower == "n" || lower == "note" || lower == "notes" {
            return ""
        }
        if lower.hasPrefix("n ") {
            return String(trimmed.dropFirst(2))
        }
        if lower.hasPrefix("note ") {
            return String(trimmed.dropFirst(5))
        }
        if lower.hasPrefix("notes ") {
            return String(trimmed.dropFirst(6))
        }
        return nil
    }

    public static func parse(payload: String, knownTemplates: Set<String>) -> NotesQuery {
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if ModuleHelp.isHelpQuery(trimmed) {
            return .help
        }
        if trimmed.isEmpty {
            return .listRecents
        }

        let lower = trimmed.lowercased()
        if lower == "daily" || lower == "today" {
            return .daily
        }

        if lower == "review week" || lower == "review" {
            return .reviewWeek
        }

        if lower == "doctor" {
            return .doctor
        }

        if lower.hasPrefix("list ") {
            let rest = String(trimmed.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
            if let filter = parseQualifier(rest) {
                return .metaSearch(filter)
            }
            return .search(trimmed)
        }

        if let filter = parseQualifier(trimmed) {
            return .metaSearch(filter)
        }

        if lower.hasPrefix("new") {
            let rest: String
            if lower == "new" {
                rest = ""
            } else if lower.hasPrefix("new ") {
                rest = String(trimmed.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
            } else {
                return .search(trimmed)
            }

            guard !rest.isEmpty else { return .search(trimmed) }

            let parts = rest.split(separator: " ", maxSplits: 1, omittingEmptySubsequences: true)
            if parts.count == 2 {
                let candidate = String(parts[0])
                let templateKey = candidate.lowercased()
                if knownTemplates.contains(templateKey) {
                    let title = String(parts[1]).trimmingCharacters(in: .whitespacesAndNewlines)
                    if !title.isEmpty {
                        return .new(title: title, template: templateKey)
                    }
                }
            }
            return .new(title: rest, template: nil)
        }

        return .search(trimmed)
    }

    private static func parseQualifier(_ text: String) -> NotesMetaFilter? {
        let lowered = text.lowercased()
        if lowered.hasPrefix("tag:") {
            let tag = String(text.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
            guard !tag.isEmpty else { return nil }
            return NotesMetaFilter(tag: tag)
        }
        if lowered.hasPrefix("type:") {
            let type = String(text.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
            guard !type.isEmpty else { return nil }
            return NotesMetaFilter(type: type)
        }
        return nil
    }
}
