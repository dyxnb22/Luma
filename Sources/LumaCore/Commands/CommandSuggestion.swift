import Foundation

public struct CommandSuggestion: Sendable, Equatable {
    public let trigger: String
    public let title: String
    public let subtitle: String?
    public let module: ModuleIdentifier
    public let example: String?

    public init(
        trigger: String,
        title: String,
        subtitle: String? = nil,
        module: ModuleIdentifier,
        example: String? = nil
    ) {
        self.trigger = trigger
        self.title = title
        self.subtitle = subtitle
        self.module = module
        self.example = example
    }
}
