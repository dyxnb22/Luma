import Foundation

public actor QuerySnapshotCache {
    private static let excludedModules: Set<ModuleIdentifier> = [
        ModuleIdentifier(rawValue: "luma.secrets"),
        ModuleIdentifier(rawValue: "luma.snippets")
    ]

    private struct Entry: Sendable {
        let snapshot: ResultSnapshot
        let storedAt: ContinuousClock.Instant
    }

    private var entries: [String: Entry] = [:]
    private let maxEntries = 64
    private let ttl: Duration = .seconds(300)

    public init() {}

    public func lookup(normalizedQuery: String, moduleGeneration: UInt64) -> ResultSnapshot? {
        let key = cacheKey(normalizedQuery: normalizedQuery, moduleGeneration: moduleGeneration)
        guard let entry = entries[key] else { return nil }
        if ContinuousClock.now - entry.storedAt > ttl {
            entries.removeValue(forKey: key)
            return nil
        }
        return entry.snapshot
    }

    public func store(normalizedQuery: String, moduleGeneration: UInt64, snapshot: ResultSnapshot) {
        guard snapshot.items.allSatisfy({ !Self.excludedModules.contains($0.id.module) }) else { return }
        let key = cacheKey(normalizedQuery: normalizedQuery, moduleGeneration: moduleGeneration)
        entries[key] = Entry(snapshot: snapshot, storedAt: .now)
        if entries.count > maxEntries {
            let oldest = entries.min { $0.value.storedAt < $1.value.storedAt }?.key
            if let oldest { entries.removeValue(forKey: oldest) }
        }
    }

    public func invalidateAll() {
        entries.removeAll(keepingCapacity: false)
    }

    private func cacheKey(normalizedQuery: String, moduleGeneration: UInt64) -> String {
        "\(moduleGeneration)|\(normalizedQuery)"
    }
}
