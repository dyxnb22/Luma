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

    public struct ActivityBucket: Sendable, Hashable {
        public let day: Date
        public let count: Int
    }

    public func activityBuckets(lastDays: Int, now: Date = Date()) -> [ActivityBucket] {
        let calendar = Calendar.current
        let start = calendar.startOfDay(for: calendar.date(byAdding: .day, value: -(lastDays - 1), to: now) ?? now)
        var buckets: [Date: Int] = [:]
        for offset in 0..<lastDays {
            if let day = calendar.date(byAdding: .day, value: offset, to: start) {
                buckets[calendar.startOfDay(for: day)] = 0
            }
        }
        for record in records.values {
            let day = calendar.startOfDay(for: record.lastUsed)
            guard day >= start else { continue }
            buckets[day, default: 0] += record.count
        }
        return buckets.keys.sorted().map { ActivityBucket(day: $0, count: buckets[$0] ?? 0) }
    }

    public func countsByModule(since: Date) -> [ModuleIdentifier: Int] {
        var totals: [ModuleIdentifier: Int] = [:]
        for record in records.values where record.lastUsed >= since {
            totals[record.id.module, default: 0] += record.count
        }
        return totals
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
