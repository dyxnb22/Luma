import Foundation

public struct CommandRouter: Sendable {
    public let registry: CommandRegistry

    public init(registry: CommandRegistry = BuiltInCommandRegistry.make()) {
        self.registry = registry
    }

    public func route(raw: String) -> CommandRoute {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            return .empty
        }

        let parts = trimmed.split(separator: " ", maxSplits: 1, omittingEmptySubsequences: false)
        let firstToken = String(parts[0]).lowercased()
        let rest = parts.count > 1 ? String(parts[1]) : ""
        let restTrimmed = rest.trimmingCharacters(in: .whitespacesAndNewlines)
        let restLower = restTrimmed.lowercased()

        if firstToken == "?" || firstToken == "help" {
            if restTrimmed.isEmpty {
                return .help(module: nil)
            }
            if let command = registry.command(forTrigger: restLower) {
                return .help(module: command.module)
            }
            return .help(module: nil)
        }

        if let command = registry.command(forTrigger: firstToken) {
            if restLower == "?" || restLower == "help" {
                return .help(module: command.module)
            }

            if command.id == "apps", !restTrimmed.isEmpty, !Self.isAppsSpecialPayload(restLower) {
                return .globalSearch(trimmed)
            }

            if restTrimmed.isEmpty {
                return .targeted(module: command.module, trigger: command.primaryTrigger, payload: "")
            }
            return .targeted(module: command.module, trigger: command.primaryTrigger, payload: restTrimmed)
        }

        let typoSuggestions = registry.suggestTriggers(for: firstToken, limit: 3)
        if !typoSuggestions.isEmpty {
            return .unknownPrefix(prefix: firstToken, remainder: restTrimmed, suggestions: typoSuggestions)
        }

        return .globalSearch(trimmed)
    }

    private static func isAppsSpecialPayload(_ payload: String) -> Bool {
        payload == "top" || payload == "?" || payload == "help"
    }
}
