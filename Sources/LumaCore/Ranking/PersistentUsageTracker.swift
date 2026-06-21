import Foundation

public actor PersistentUsageTracker: UsageTracking {
    private let url: URL
    private var records: [ResultID: UsageRecord]

    public init(url: URL) {
        self.url = url
        self.records = (try? Self.load(from: url)) ?? [:]
    }

    public static func defaultTracker(fileManager: FileManager = .default) -> PersistentUsageTracker {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return PersistentUsageTracker(url: base.appendingPathComponent("Luma/usage-records.json"))
    }

    public func snapshot() -> [ResultID: UsageRecord] {
        records
    }

    public func record(_ id: ResultID, at date: Date = Date()) {
        var record = records[id] ?? UsageRecord(id: id, count: 0, lastUsed: date)
        record.count += 1
        record.lastUsed = date
        records[id] = record
        persist()
    }

    public func recent(limit: Int = 8) -> [UsageRecord] {
        Array(records.values.sorted {
            if $0.lastUsed == $1.lastUsed { return $0.count > $1.count }
            return $0.lastUsed > $1.lastUsed
        }.prefix(limit))
    }

    private func persist() {
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoded = records.map { StoredUsageRecord(id: $0.key, record: $0.value) }
        if let data = try? JSONEncoder().encode(encoded) {
            try? data.write(to: url, options: .atomic)
        }
    }

    private static func load(from url: URL) throws -> [ResultID: UsageRecord] {
        let data = try Data(contentsOf: url)
        let encoded = try JSONDecoder().decode([StoredUsageRecord].self, from: data)
        return Dictionary(uniqueKeysWithValues: encoded.map { ($0.id, $0.record) })
    }
}

private struct StoredUsageRecord: Codable {
    let id: ResultID
    let record: UsageRecord
}
