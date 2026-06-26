import Foundation

public struct StorableResultItem: Codable, Sendable {
    public let id: ResultID
    public let title: String
    public let subtitle: String?
    public let icon: IconRef
    public let primaryAction: Action
    public let secondaryActions: [Action]
    public let basePriority: Int

    public init(_ item: ResultItem) {
        id = item.id
        title = item.title
        subtitle = item.subtitle
        icon = item.icon
        primaryAction = item.primaryAction
        secondaryActions = item.secondaryActions
        basePriority = item.rankingHints.basePriority
    }

    public func resultItem() -> ResultItem {
        ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: icon,
            primaryAction: primaryAction,
            secondaryActions: secondaryActions,
            rankingHints: RankingHints(basePriority: basePriority)
        )
    }
}

public actor UsageResultCache {
    private static let excludedModules: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.clipboard"),
        ModuleIdentifier(rawValue: "luma.secrets"),
        ModuleIdentifier(rawValue: "luma.snippets")
    ]

    private let url: URL
    private var items: [ResultID: StorableResultItem]

    public init(url: URL) {
        self.url = url
        let loaded = (try? Self.load(from: url)) ?? [:]
        let cleaned = loaded.filter { !Self.shouldExclude($0.key) }
        self.items = cleaned
        if cleaned.count != loaded.count {
            Self.write(cleaned, to: url)
        }
    }

    public static func defaultCache(fileManager: FileManager = .default) -> UsageResultCache {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return UsageResultCache(url: base.appendingPathComponent("Luma/usage-result-cache.json"))
    }

    public func store(_ item: ResultItem) {
        guard !Self.shouldExclude(item.id) else { return }
        items[item.id] = StorableResultItem(item)
        persist()
    }

    public func item(for id: ResultID) -> ResultItem? {
        items[id]?.resultItem()
    }

    private static func shouldExclude(_ id: ResultID) -> Bool {
        excludedModules.contains(id.module)
    }

    private func persist() {
        Self.write(items, to: url)
    }

    private static func write(_ items: [ResultID: StorableResultItem], to url: URL) {
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoded = Array(items.values)
        if let data = try? JSONEncoder().encode(encoded) {
            try? data.write(to: url, options: .atomic)
        }
    }

    private static func load(from url: URL) throws -> [ResultID: StorableResultItem] {
        let data = try Data(contentsOf: url)
        let encoded = try JSONDecoder().decode([StorableResultItem].self, from: data)
        return Dictionary(uniqueKeysWithValues: encoded.map { ($0.id, $0) })
    }
}
