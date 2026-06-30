import Foundation

public struct CommandRegistry: Sendable {
    public static let defaultPlaceholder = "Search or type a command…"

    private let commands: [CommandDefinition]
    private let triggerMap: [String: CommandDefinition]
    private let moduleMap: [ModuleIdentifier: CommandDefinition]

    public init(_ commands: [CommandDefinition]) {
        self.commands = commands
        var map: [String: CommandDefinition] = [:]
        var modules: [ModuleIdentifier: CommandDefinition] = [:]
        for command in commands {
            for trigger in command.allTriggers {
                map[trigger.lowercased()] = command
            }
            if modules[command.module] == nil {
                modules[command.module] = command
            }
        }
        self.triggerMap = map
        self.moduleMap = modules
    }

    public var allCommands: [CommandDefinition] {
        commands
    }

    public var discoverableCommands: [CommandDefinition] {
        discoverableCommands(usage: [:])
    }

    public func discoverableCommands(usage: [String: Int]) -> [CommandDefinition] {
        commands
            .filter(\.isDiscoverable)
            .sorted { lhs, rhs in
                let leftCount = usage[lhs.primaryTrigger.lowercased(), default: 0]
                let rightCount = usage[rhs.primaryTrigger.lowercased(), default: 0]
                if leftCount != rightCount {
                    return leftCount > rightCount
                }
                if lhs.discoverPriority != rhs.discoverPriority {
                    return lhs.discoverPriority < rhs.discoverPriority
                }
                return lhs.primaryTrigger.localizedCaseInsensitiveCompare(rhs.primaryTrigger) == .orderedAscending
            }
    }

    public func command(forTrigger trigger: String) -> CommandDefinition? {
        triggerMap[trigger.lowercased()]
    }

    public func command(forModule module: ModuleIdentifier) -> CommandDefinition? {
        moduleMap[module]
    }

    public func sectionTitle(for module: ModuleIdentifier) -> String {
        if let command = moduleMap[module] {
            return command.sectionTitle
        }
        return module.rawValue
            .replacingOccurrences(of: "luma.", with: "")
            .replacingOccurrences(of: "-", with: " ")
            .uppercased()
    }

    public func hint(for query: String) -> CommandHint? {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        let parts = trimmed.split(separator: " ", maxSplits: 1, omittingEmptySubsequences: false)
        let firstToken = String(parts[0]).lowercased()
        let rest = parts.count > 1 ? String(parts[1]).trimmingCharacters(in: .whitespacesAndNewlines) : ""

        if firstToken == "?" || firstToken == "help" {
            guard !rest.isEmpty else { return nil }
            let helpTarget = rest.split(separator: " ", maxSplits: 1).first.map(String.init) ?? rest
            guard let command = command(forTrigger: helpTarget.lowercased()) else { return nil }
            return command.commandHint
        }

        guard let command = command(forTrigger: firstToken) else { return nil }
        return command.commandHint
    }

    public func parsedCommand(for raw: String, route: CommandRoute) -> ParsedCommand? {
        switch route {
        case .targeted(let module, let trigger, let payload):
            return ParsedCommand(trigger: trigger, payload: payload, module: module)
        case .help(let module?):
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            let parts = trimmed.split(separator: " ", maxSplits: 1, omittingEmptySubsequences: false)
            let trigger = command(forModule: module)?.primaryTrigger ?? String(parts[0])
            let payload = parts.count > 1 ? String(parts[1]).trimmingCharacters(in: .whitespacesAndNewlines) : ""
            return ParsedCommand(trigger: trigger, payload: payload, module: module)
        case .empty, .globalSearch, .help(nil), .suggestion, .unknownPrefix:
            return nil
        }
    }

    public func placeholder(for query: String) -> String {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return Self.defaultPlaceholder }

        let lower = trimmed.lowercased()
        if let first = lower.split(separator: " ", maxSplits: 1).first,
           let command = command(forTrigger: String(first)) {
            return command.placeholder
        }

        return Self.defaultPlaceholder
    }

    public func landingSuggestion(for command: CommandDefinition) -> CommandSuggestion {
        CommandSuggestion(
            trigger: command.primaryTrigger,
            title: command.title,
            subtitle: command.placeholder,
            module: command.module,
            example: command.examples.first
        )
    }

    public func globalHelpSuggestions() -> [CommandSuggestion] {
        discoverableCommands.map { command in
            CommandSuggestion(
                trigger: command.primaryTrigger,
                title: command.title,
                subtitle: command.examples.first,
                module: command.module,
                example: command.examples.first
            )
        }
    }

    public func suggestTriggers(for typo: String, limit: Int = 3) -> [CommandSuggestion] {
        let normalized = typo.lowercased()
        guard !normalized.isEmpty else { return [] }

        let scored = commands.filter(\.isDiscoverable).flatMap { command -> [(Double, CommandSuggestion)] in
            Self.typoCandidateTriggers(for: command).compactMap { trigger -> (Double, CommandSuggestion)? in
                guard Self.looksLikeCommandTypo(normalized, trigger) else { return nil }
                let distance = Self.editDistance(normalized, trigger)
                guard distance > 0, distance <= 2 else { return nil }
                if distance == 2 {
                    guard normalized.count == trigger.count,
                          normalized.count >= 3,
                          normalized.first == trigger.first else { return nil }
                }
                let score = 1.0 / Double(distance + 1)
                return (
                    score,
                    CommandSuggestion(
                        trigger: command.primaryTrigger,
                        title: command.title,
                        subtitle: "Did you mean \"\(command.primaryTrigger)\"?",
                        module: command.module,
                        example: command.examples.first
                    )
                )
            }
        }
        .sorted { $0.0 > $1.0 }

        var seen = Set<String>()
        var suggestions: [CommandSuggestion] = []
        for (_, suggestion) in scored {
            guard seen.insert(suggestion.trigger).inserted else { continue }
            suggestions.append(suggestion)
            if suggestions.count >= limit { break }
        }
        return suggestions
    }

    private static func typoCandidateTriggers(for command: CommandDefinition) -> [String] {
        [command.primaryTrigger] + command.aliases.filter { $0.count >= 4 }
    }

    private static func looksLikeCommandTypo(_ input: String, _ trigger: String) -> Bool {
        let lengthDelta = abs(input.count - trigger.count)
        if trigger.count == 1 {
            return input.count <= 2
        }
        return lengthDelta <= 1
    }

    private static func editDistance(_ lhs: String, _ rhs: String) -> Int {
        let a = Array(lhs)
        let b = Array(rhs)
        if a.isEmpty { return b.count }
        if b.isEmpty { return a.count }

        var previous = Array(0...b.count)
        for i in 1...a.count {
            var current = [i]
            for j in 1...b.count {
                let cost = a[i - 1] == b[j - 1] ? 0 : 1
                current.append(
                    min(
                        current[j - 1] + 1,
                        previous[j] + 1,
                        previous[j - 1] + cost
                    )
                )
            }
            previous = current
        }
        return previous[b.count]
    }
}

