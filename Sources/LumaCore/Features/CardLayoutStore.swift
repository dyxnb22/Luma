import Foundation

public struct CardLayoutStore: Sendable {
    private let url: URL

    public init(url: URL) {
        self.url = url
    }

    public static func defaultStore(fileManager: FileManager = .default) -> CardLayoutStore {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return CardLayoutStore(url: base.appendingPathComponent("Luma/card-layout.json"))
    }

    public func load(cards: [FeatureCard]) -> [FeatureCard] {
        guard let data = try? Data(contentsOf: url),
              let layout = try? JSONDecoder().decode([String: CardPosition].self, from: data) else {
            return cards
        }
        return cards.map { card in
            var updated = card
            if let position = layout[card.id.rawValue] {
                updated.position = position
            }
            return updated
        }
    }

    public func save(cards: [FeatureCard]) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let layout = Dictionary(uniqueKeysWithValues: cards.map { ($0.id.rawValue, $0.position) })
        let data = try JSONEncoder().encode(layout)
        try data.write(to: url, options: .atomic)
    }

    public func save(position: CardPosition, for id: ModuleIdentifier) throws {
        var layout: [String: CardPosition] = [:]
        if let data = try? Data(contentsOf: url),
           let existing = try? JSONDecoder().decode([String: CardPosition].self, from: data) {
            layout = existing
        }
        layout[id.rawValue] = position
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let data = try JSONEncoder().encode(layout)
        try data.write(to: url, options: .atomic)
    }
}
