import Foundation

public enum SearchEmptyState {
    public static func message(for route: CommandRoute, query: String, registry: CommandRegistry) -> String {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        switch route {
        case .empty:
            return L10n.tr("search.empty.typeToSearch")
        case .help:
            return withClearHint(L10n.tr("search.empty.help"), query: query)
        case .targeted(let module, _, _):
            let section = registry.sectionTitle(for: module)
            if trimmed.isEmpty {
                return withClearHint(L10n.tr("search.empty.noItemsInSection", section), query: query)
            }
            return withClearHint(L10n.tr("search.empty.noMatchesInSection", section, trimmed), query: query)
        case .unknownPrefix(let prefix, _, let suggestions):
            if let first = suggestions.first {
                return withClearHint(L10n.tr("search.empty.unknownDidYouMean", prefix, first.trigger), query: query)
            }
            return withClearHint(L10n.tr("search.empty.unknownHelp", prefix, prefix), query: query)
        case .globalSearch:
            if trimmed.count < 2 {
                return withClearHint(L10n.tr("search.empty.globalMinChars"), query: query)
            }
            return withClearHint(L10n.tr("search.empty.globalNoMatches", trimmed), query: query)
        case .suggestion:
            return withClearHint(L10n.tr("search.empty.suggestion"), query: query)
        }
    }

    private static func withClearHint(_ message: String, query: String) -> String {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return message }
        return "\(message) \(L10n.tr("search.empty.pressEscClear"))"
    }
}
