import Foundation

public struct CommandRouter: Sendable {
    public let registry: CommandRegistry

    public init(registry: CommandRegistry = CommandRegistry([])) {
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

            if command.bareBehavior == .globalSearchShadow,
               !restTrimmed.isEmpty,
               !command.bareReservedPayloads.contains(where: { $0.caseInsensitiveCompare(restLower) == .orderedSame }) {
                return .globalSearch(trimmed)
            }

            if restTrimmed.isEmpty {
                return .targeted(module: command.module, trigger: command.primaryTrigger, payload: "")
            }
            return .targeted(module: command.module, trigger: command.primaryTrigger, payload: restTrimmed)
        }

        if firstToken.count == 1 {
            return .globalSearch(trimmed)
        }

        let typoSuggestions = registry.suggestTriggers(for: firstToken, limit: 3)
        if !typoSuggestions.isEmpty {
            return .unknownPrefix(prefix: firstToken, remainder: restTrimmed, suggestions: typoSuggestions)
        }

        return .globalSearch(trimmed)
    }

    /// True when Return should open the module detail panel instead of running the selected row.
    public func isBareOpenDetailReturn(raw: String) -> Bool {
        let route = route(raw: raw)
        guard case .targeted(let module, _, let payload) = route else { return false }
        if module.rawValue == "luma.snippets" {
            let lower = payload.lowercased()
            return lower == "new" || lower.hasPrefix("new ")
        }
        guard let command = registry.command(forModule: module),
              command.bareBehavior == .openDetail else { return false }
        if payload.isEmpty { return true }
        if module.rawValue == "luma.wordbook",
           payload.compare("review", options: .caseInsensitive) == .orderedSame {
            return true
        }
        return false
    }
}
