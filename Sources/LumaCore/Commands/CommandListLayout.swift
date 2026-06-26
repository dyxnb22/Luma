import Foundation

public struct ParsedCommand: Sendable, Hashable {
    public let trigger: String
    public let payload: String
    public let module: ModuleIdentifier

    public init(trigger: String, payload: String, module: ModuleIdentifier) {
        self.trigger = trigger
        self.payload = payload
        self.module = module
    }
}

public struct CommandHint: Sendable, Equatable {
    public let usageFormat: String
    public let description: String
    public let example: String?

    public init(usageFormat: String, description: String, example: String? = nil) {
        self.usageFormat = usageFormat
        self.description = description
        self.example = example
    }
}

public struct ResultSection: Sendable, Equatable {
    public let title: String
    public let items: [ResultItem]

    public init(title: String, items: [ResultItem]) {
        self.title = title
        self.items = items
    }
}

public enum ResultListLayout: Sendable, Equatable {
    case flat
    case sectioned([ResultSection])
}

public enum CommandListLayout {
    public static func build(
        items: [ResultItem],
        route: CommandRoute,
        registry: CommandRegistry
    ) -> ResultListLayout {
        switch route {
        case .targeted(let module, _, _):
            let title = registry.sectionTitle(for: module)
            return .sectioned([ResultSection(title: title, items: items)])
        case .help(let module?):
            let title = registry.sectionTitle(for: module)
            return .sectioned([ResultSection(title: title, items: items)])
        case .globalSearch:
            return .sectioned(groupByModule(items, registry: registry))
        case .empty, .help(nil), .suggestion, .unknownPrefix:
            return .flat
        }
    }

    private static func groupByModule(_ items: [ResultItem], registry: CommandRegistry) -> [ResultSection] {
        var order: [ModuleIdentifier] = []
        var buckets: [ModuleIdentifier: [ResultItem]] = [:]
        for item in items {
            if buckets[item.id.module] == nil {
                order.append(item.id.module)
                buckets[item.id.module] = []
            }
            buckets[item.id.module, default: []].append(item)
        }
        return order.compactMap { module in
            guard let sectionItems = buckets[module], !sectionItems.isEmpty else { return nil }
            return ResultSection(title: registry.sectionTitle(for: module), items: sectionItems)
        }
    }
}
