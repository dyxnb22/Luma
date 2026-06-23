import Foundation
import LumaCore

/// Contextual search placeholder strings keyed by module trigger prefix.
public enum ModuleSearchHints {
    public static let `default` = "Search"

    public static func placeholder(for query: String) -> String {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "t" || lower.hasPrefix("t ") || lower == "todo" || lower.hasPrefix("todo ") {
            return "Add a task or list today's reminders"
        }
        if lower == "s" || lower.hasPrefix("s ") || lower == "snip" || lower.hasPrefix("snip ") {
            return "Find a snippet"
        }
        if lower == "m" || lower.hasPrefix("m ") || lower == "media" || lower.hasPrefix("media ") {
            return "Log media or search history"
        }
        if lower.hasPrefix("word") {
            return "Search vocabulary or start review"
        }
        if lower.hasPrefix("secret") {
            return "Search saved secrets"
        }
        if lower.hasPrefix("clip") {
            return "Search clipboard history"
        }
        if lower == "n" || lower.hasPrefix("n ") || lower.hasPrefix("note") {
            return "New note, daily, or search by filename"
        }
        if lower == "tr" || lower.hasPrefix("tr ") || lower.hasPrefix("translate") {
            return "Text to translate"
        }
        return Self.default
    }
}
