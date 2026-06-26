import Foundation

public struct ScriptCommand: Sendable, Codable, Hashable, Identifiable {
    public var id: String
    public var title: String
    public var trigger: String
    public var exec: String
    public var args: [String]
    public var cwd: String?
    public var timeoutSec: Int
    public var showOutput: String

    public init(
        id: String,
        title: String,
        trigger: String,
        exec: String = "/bin/zsh",
        args: [String] = [],
        cwd: String? = nil,
        timeoutSec: Int = 600,
        showOutput: String = "notification"
    ) {
        self.id = id
        self.title = title
        self.trigger = trigger
        self.exec = exec
        self.args = args
        self.cwd = cwd
        self.timeoutSec = timeoutSec
        self.showOutput = showOutput
    }
}

public struct CommandsConfig: Sendable, Codable, Equatable {
    public var commands: [ScriptCommand]

    public static let empty = CommandsConfig(commands: [])

    public init(commands: [ScriptCommand]) {
        self.commands = commands
    }
}

public enum CommandsAction: Codable, Sendable, Hashable {
    case run(id: String)
    case revealConfig
    case doctor
}
