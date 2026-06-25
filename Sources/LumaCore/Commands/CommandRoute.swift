import Foundation

public enum CommandRoute: Sendable, Equatable {
    case empty
    case globalSearch(String)
    case targeted(module: ModuleIdentifier, trigger: String, payload: String)
    case help(module: ModuleIdentifier?)
    case suggestion([CommandSuggestion])
    case unknownPrefix(prefix: String, remainder: String, suggestions: [CommandSuggestion])
}
