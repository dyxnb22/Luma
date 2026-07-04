import Foundation

public struct HomeGuideEntryRow: Sendable, Equatable {
    public let commandID: String
    public let moduleName: String
    public let trigger: String
    public let summary: String

    public init(commandID: String, moduleName: String, trigger: String, summary: String) {
        self.commandID = commandID
        self.moduleName = moduleName
        self.trigger = trigger
        self.summary = summary
    }
}

/// Compact module entry rows for the empty-query home guide (right column).
public enum HomeGuideCatalog {
    private static let excludedModules: Set<String> = ["luma.apps", "luma.commands"]

    public static func entryRows(
        from commands: [CommandDefinition],
        localize: (String) -> String
    ) -> [HomeGuideEntryRow] {
        var primaryByModule: [ModuleIdentifier: CommandDefinition] = [:]
        for command in commands where command.isDiscoverable && !excludedModules.contains(command.module.rawValue) {
            if let existing = primaryByModule[command.module] {
                let shouldReplace = command.discoverPriority > existing.discoverPriority
                    || (
                        command.discoverPriority == existing.discoverPriority
                            && command.primaryTrigger.localizedCaseInsensitiveCompare(existing.primaryTrigger) == .orderedAscending
                    )
                if shouldReplace {
                    primaryByModule[command.module] = command
                }
            } else {
                primaryByModule[command.module] = command
            }
        }

        return primaryByModule.values
            .sorted {
                if $0.discoverPriority != $1.discoverPriority {
                    return $0.discoverPriority > $1.discoverPriority
                }
                return $0.primaryTrigger.localizedCaseInsensitiveCompare($1.primaryTrigger) == .orderedAscending
            }
            .map { command in
                let nameKey = "home.guide.name.\(command.id)"
                let blurbKey = "home.guide.blurb.\(command.id)"
                let moduleName = localizedOrFallback(localize(nameKey), nameKey, fallback: command.title)
                let summary = localizedOrFallback(localize(blurbKey), blurbKey, fallback: command.resolvedDescription)
                return HomeGuideEntryRow(
                    commandID: command.id,
                    moduleName: moduleName,
                    trigger: command.primaryTrigger,
                    summary: summary
                )
            }
    }

    private static func localizedOrFallback(_ value: String, _ key: String, fallback: String) -> String {
        value == key ? fallback : value
    }
}
