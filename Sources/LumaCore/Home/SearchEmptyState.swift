import Foundation

public enum SearchEmptyState {
    public static func message(for route: CommandRoute, query: String, registry: CommandRegistry) -> String {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        switch route {
        case .empty:
            return "Type to search apps, commands, and modules"
        case .help:
            return "Type a command followed by ? for help (e.g. clip ?)"
        case .targeted(let module, _, _):
            let section = registry.sectionTitle(for: module)
            if trimmed.isEmpty {
                return "No items in \(section). Add data or try a more specific query."
            }
            return "No matches in \(section) for “\(trimmed)”. Check spelling or module settings."
        case .unknownPrefix(let prefix, _, let suggestions):
            if let first = suggestions.first {
                return "Unknown command “\(prefix)”. Did you mean \(first.trigger)?"
            }
            return "Unknown command “\(prefix)”. Type ? for help."
        case .globalSearch:
            if trimmed.count < 2 {
                return "Type at least 2 characters for global search, or use a command prefix (clip, note, t)."
            }
            return "No matches for “\(trimmed)”. Enable modules in Settings or try a command prefix."
        case .suggestion:
            return "Select a suggestion or press Return"
        }
    }
}
