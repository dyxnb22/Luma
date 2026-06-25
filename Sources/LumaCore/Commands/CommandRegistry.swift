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
        guard let command = command(forTrigger: firstToken) else { return nil }

        return CommandHint(
            trigger: command.primaryTrigger,
            title: command.title,
            example: command.examples.first
        )
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

public enum BuiltInCommandRegistry {
    private enum ID {
        static let apps = ModuleIdentifier(rawValue: "luma.apps")
        static let projects = ModuleIdentifier(rawValue: "luma.projects")
        static let windowLayouts = ModuleIdentifier(rawValue: "luma.window-layouts")
        static let translate = ModuleIdentifier(rawValue: "luma.translate")
        static let clipboard = ModuleIdentifier(rawValue: "luma.clipboard")
        static let todo = ModuleIdentifier(rawValue: "luma.todo")
        static let notes = ModuleIdentifier(rawValue: "luma.notes")
        static let snippets = ModuleIdentifier(rawValue: "luma.snippets")
        static let media = ModuleIdentifier(rawValue: "luma.media")
        static let secrets = ModuleIdentifier(rawValue: "luma.secrets")
        static let wordbook = ModuleIdentifier(rawValue: "luma.wordbook")
        static let commands = ModuleIdentifier(rawValue: "luma.commands")
        static let events = ModuleIdentifier(rawValue: "luma.events")
    }

    public static func make() -> CommandRegistry {
        CommandRegistry([
            CommandDefinition(
                id: "apps",
                module: ID.apps,
                title: "Apps",
                primaryTrigger: "app",
                aliases: ["apps"],
                placeholder: "Search apps or type app top for memory usage",
                examples: ["chrome", "app top"],
                sectionTitle: "APPS",
                helpLines: [
                    "Type app name — launch or focus",
                    "app top — memory usage leaders (quit from row)",
                    "app ? — this help"
                ],
                discoverPriority: 110,
                isDiscoverable: true
            ),
            CommandDefinition(
                id: "projects",
                module: ID.projects,
                title: "Open Project",
                primaryTrigger: "p",
                aliases: ["proj", "project"],
                placeholder: "Open a project in Cursor, VS Code, Finder, or Terminal",
                examples: ["p luma", "proj api"],
                sectionTitle: "PROJECTS",
                helpLines: [
                    "p — recent projects",
                    "p luma — open in preferred IDE",
                    "p ? — this help"
                ],
                discoverPriority: 10
            ),
            CommandDefinition(
                id: "window-layouts",
                module: ID.windowLayouts,
                title: "Move Window",
                primaryTrigger: "win",
                aliases: ["wl", "layout"],
                placeholder: "Move focused window: left, right, max, center",
                examples: ["win left", "wl center"],
                sectionTitle: "WINDOWS",
                helpLines: [
                    "win left / right / max / center — move focused window",
                    "wl and layout are aliases",
                    "win ? — this help"
                ],
                discoverPriority: 20
            ),
            CommandDefinition(
                id: "translate",
                module: ID.translate,
                title: "Translate",
                primaryTrigger: "tr",
                aliases: ["translate"],
                placeholder: "Text to translate",
                examples: ["tr hello"],
                sectionTitle: "TRANSLATE",
                helpLines: [
                    "tr <text> — translate to target language",
                    "translate <text> — same command",
                    "Open detail for language settings"
                ],
                discoverPriority: 30
            ),
            CommandDefinition(
                id: "clipboard",
                module: ID.clipboard,
                title: "Clipboard",
                primaryTrigger: "clip",
                aliases: ["cb"],
                placeholder: "Search clipboard history",
                examples: ["clip jwt", "clip image"],
                sectionTitle: "CLIPBOARD",
                helpLines: [
                    "clip — search clipboard history",
                    "clip https — filter links",
                    "clip ? — this help"
                ],
                discoverPriority: 40
            ),
            CommandDefinition(
                id: "todo",
                module: ID.todo,
                title: "Todo",
                primaryTrigger: "t",
                aliases: ["todo"],
                placeholder: "Add a task or list today's reminders",
                examples: ["t buy milk tomorrow"],
                sectionTitle: "TODO",
                helpLines: [
                    "t — list today's due reminders",
                    "t buy milk — create reminder (Inbox when no date)",
                    "t pay rent tomorrow 9:00 — create with due date",
                    "Return on a row — mark complete",
                    "t ? — this help"
                ],
                discoverPriority: 50
            ),
            CommandDefinition(
                id: "notes",
                module: ID.notes,
                title: "Notes",
                primaryTrigger: "n",
                aliases: ["note", "notes"],
                placeholder: "New note, daily, or search by filename",
                examples: ["n daily", "n new idea"],
                sectionTitle: "NOTES",
                helpLines: [
                    "n — recent notes",
                    "n <query> — fuzzy find by filename",
                    "n new <title> — create in Inbox and open",
                    "n daily — open or create today's daily note",
                    "n review week — weekly review with modified notes",
                    "n ? — this help"
                ],
                discoverPriority: 60
            ),
            CommandDefinition(
                id: "snippets",
                module: ID.snippets,
                title: "Snippets",
                primaryTrigger: "s",
                aliases: ["snip"],
                placeholder: "Find a snippet",
                examples: ["s git"],
                sectionTitle: "SNIPPETS",
                helpLines: [
                    "s — top snippets by frecency",
                    "s git — fuzzy search title/tags/content",
                    "Return — copy snippet",
                    "s ? — this help"
                ],
                discoverPriority: 70
            ),
            CommandDefinition(
                id: "records",
                module: ID.media,
                title: "Log Record",
                primaryTrigger: "rec",
                aliases: ["record", "log", "m", "media"],
                placeholder: "Log a book, movie, show, anime, or game",
                examples: ["rec 三体 book done 9 #sci-fi"],
                sectionTitle: "RECORDS",
                helpLines: [
                    "rec — recent items + open logbook",
                    "rec log — full Records view",
                    "rec 三体 book done 9 #sci-fi — quick capture DSL",
                    "rec 三体 — search or partial capture",
                    "rec ? — this help"
                ],
                discoverPriority: 80
            ),
            CommandDefinition(
                id: "secrets",
                module: ID.secrets,
                title: "Secrets",
                primaryTrigger: "sec",
                aliases: ["secret", "secrets"],
                placeholder: "Search saved secrets",
                examples: ["sec aws"],
                sectionTitle: "SECRETS",
                helpLines: [
                    "sec unlock — unlock vault",
                    "sec aws — search by label",
                    "Return — copy secret (auto-clear pasteboard)",
                    "sec ? — this help"
                ],
                discoverPriority: 90
            ),
            CommandDefinition(
                id: "wordbook",
                module: ID.wordbook,
                title: "Wordbook",
                primaryTrigger: "word",
                aliases: ["wb"],
                placeholder: "Search vocabulary or start review",
                examples: ["word review"],
                sectionTitle: "WORDBOOK",
                helpLines: [
                    "word — today's due words + start review",
                    "word abandon — search term/meaning",
                    "Start Review — opens review panel",
                    "word ? — this help"
                ],
                discoverPriority: 100
            ),
            CommandDefinition(
                id: "events",
                module: ID.events,
                title: "Calendar",
                primaryTrigger: "e",
                aliases: ["event"],
                placeholder: "List or create calendar events",
                examples: ["e meet john tomorrow 14:00"],
                sectionTitle: "EVENTS",
                helpLines: [
                    "e — list today's calendar events",
                    "e meet john tomorrow 14:00 — create event",
                    "Return on capture row — save to Calendar",
                    "e ? — this help"
                ],
                discoverPriority: 105
            ),
            CommandDefinition(
                id: "settings",
                module: ID.commands,
                title: "Settings",
                primaryTrigger: "settings",
                aliases: ["prefs"],
                placeholder: "Open Luma preferences",
                examples: ["settings"],
                sectionTitle: "COMMANDS",
                helpLines: [
                    "settings — open Luma preferences",
                    "open-settings — same from command mode",
                    "reload-modules — refresh module registry",
                    "quit — exit Luma"
                ],
                discoverPriority: 120,
                isDiscoverable: true
            ),
            CommandDefinition(
                id: "open-settings",
                module: ID.commands,
                title: "Open Settings",
                primaryTrigger: "open-settings",
                placeholder: "Open Luma preferences",
                examples: ["open-settings"],
                sectionTitle: "COMMANDS",
                helpLines: ["Open Luma preferences"],
                isDiscoverable: false
            ),
            CommandDefinition(
                id: "reload-modules",
                module: ID.commands,
                title: "Reload Modules",
                primaryTrigger: "reload-modules",
                placeholder: "Refresh module registry",
                examples: ["reload-modules"],
                sectionTitle: "COMMANDS",
                helpLines: ["Reload module registry and warm up modules"],
                isDiscoverable: false
            ),
            CommandDefinition(
                id: "quit",
                module: ID.commands,
                title: "Quit Luma",
                primaryTrigger: "quit",
                placeholder: "Exit Luma",
                examples: ["quit"],
                sectionTitle: "COMMANDS",
                helpLines: ["Quit Luma"],
                isDiscoverable: false
            )
        ])
    }
}
