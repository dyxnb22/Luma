import Foundation

public struct CommandDefinition: Sendable, Equatable {
    public let id: String
    public let module: ModuleIdentifier
    public let title: String
    public let primaryTrigger: String
    public let aliases: [String]
    public let placeholder: String
    public let examples: [String]
    public let sectionTitle: String
    public let helpLines: [String]
    public let discoverPriority: Int
    public let isDiscoverable: Bool

    public init(
        id: String,
        module: ModuleIdentifier,
        title: String,
        primaryTrigger: String,
        aliases: [String] = [],
        placeholder: String,
        examples: [String] = [],
        sectionTitle: String? = nil,
        helpLines: [String] = [],
        discoverPriority: Int = 100,
        isDiscoverable: Bool = true
    ) {
        self.id = id
        self.module = module
        self.title = title
        self.primaryTrigger = primaryTrigger
        self.aliases = aliases
        self.placeholder = placeholder
        self.examples = examples
        self.sectionTitle = sectionTitle ?? title.uppercased()
        self.helpLines = helpLines
        self.discoverPriority = discoverPriority
        self.isDiscoverable = isDiscoverable
    }

    public var allTriggers: [String] {
        [primaryTrigger] + aliases
    }
}
