import Foundation

public extension ModuleIdentifier {
    static let commandEntry = ModuleIdentifier(rawValue: "luma.command-entry")
}

public enum CommandEntryResults {
    public static func globalHelp(registry: CommandRegistry, usage: [String: Int] = [:]) -> [ResultItem] {
        var rows = registry.discoverableCommands(usage: usage).map { helpRow(for: $0) }
        rows.append(footerHelpRow())
        return rows
    }

    public static func suggestionRows(_ suggestions: [CommandSuggestion]) -> [ResultItem] {
        suggestions.map { suggestionRow($0, keyPrefix: "suggestion") }
    }

    public static func unknownPrefixRows(
        prefix: String,
        suggestions: [CommandSuggestion],
        remainder: String = ""
    ) -> [ResultItem] {
        var rows: [ResultItem] = []
        if let best = suggestions.first {
            let replacement = remainder.isEmpty
                ? "\(best.trigger) "
                : "\(best.trigger) \(remainder)"
            rows.append(
                ResultItem(
                    id: ResultID(module: .commandEntry, key: "replace:\(replacement)"),
                    title: "Unknown command \"\(prefix)\"",
                    titleAttributed: AttributedString("Unknown command \"\(prefix)\""),
                    subtitle: "Did you mean \"\(best.trigger)\"?",
                    icon: .symbol("exclamationmark.triangle"),
                    primaryAction: Action(
                        id: ActionID(module: .commandEntry, key: "replace"),
                        title: "Use \(best.trigger)",
                        kind: .replaceQuery(replacement)
                    ),
                    rankingHints: RankingHints(basePriority: 100),
                    rowKind: .informational
                )
            )
        }
        rows.append(contentsOf: suggestionRows(suggestions))
        return rows
    }

    private static func helpRow(for command: CommandDefinition) -> ResultItem {
        let subtitle = command.resolvedDescription
        let query = "\(command.primaryTrigger) ?"
        return ResultItem(
            id: ResultID(module: .commandEntry, key: "help.\(command.primaryTrigger)"),
            title: "\(command.primaryTrigger)  \(command.title)",
            titleAttributed: AttributedString("\(command.primaryTrigger)  \(command.title)"),
            subtitle: subtitle,
            icon: .symbol("command"),
            primaryAction: Action(
                id: ActionID(module: command.module, key: "help.open"),
                title: query,
                kind: .replaceQuery(query)
            ),
            rankingHints: RankingHints(basePriority: 90),
            rowKind: .informational
        )
    }

    private static func footerHelpRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: .commandEntry, key: "help.footer"),
            title: "Type <prefix> ? for detailed module help",
            titleAttributed: AttributedString("Type <prefix> ? for detailed module help"),
            subtitle: "Example: rec ?",
            icon: .symbol("questionmark.circle"),
            primaryAction: Action(
                id: ActionID(module: .commandEntry, key: "help.footer.example"),
                title: "rec ?",
                kind: .replaceQuery("rec ?")
            ),
            rankingHints: RankingHints(basePriority: 0),
            rowKind: .informational
        )
    }

    private static func suggestionRow(_ suggestion: CommandSuggestion, keyPrefix: String) -> ResultItem {
        let subtitle = suggestion.example ?? suggestion.subtitle
        let query = "\(suggestion.trigger) "
        return ResultItem(
            id: ResultID(module: .commandEntry, key: "\(keyPrefix).\(suggestion.trigger)"),
            title: "\(suggestion.trigger)  \(suggestion.title)",
            titleAttributed: AttributedString("\(suggestion.trigger)  \(suggestion.title)"),
            subtitle: subtitle,
            icon: .symbol("command"),
            primaryAction: Action(
                id: ActionID(module: suggestion.module, key: "\(keyPrefix).select"),
                title: suggestion.title,
                kind: .replaceQuery(query)
            ),
            rankingHints: RankingHints(basePriority: 90),
            rowKind: .informational
        )
    }
}
