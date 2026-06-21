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
    private let url: URL
    private var items: [ResultID: StorableResultItem]

    public init(url: URL) {
        self.url = url
        self.items = (try? Self.load(from: url)) ?? [:]
    }

    public static func defaultCache(fileManager: FileManager = .default) -> UsageResultCache {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return UsageResultCache(url: base.appendingPathComponent("Luma/usage-result-cache.json"))
    }

    public func store(_ item: ResultItem) {
        items[item.id] = StorableResultItem(item)
        persist()
    }

    public func item(for id: ResultID) -> ResultItem? {
        items[id]?.resultItem()
    }

    private func persist() {
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
