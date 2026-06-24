import Foundation

struct ClipboardHistorySnapshot: Codable, Sendable {
    static let currentSchemaVersion = 1

    var schemaVersion: Int
    var entries: [ClipboardEntry]

    init(entries: [ClipboardEntry], schemaVersion: Int = Self.currentSchemaVersion) {
        self.schemaVersion = schemaVersion
        self.entries = entries
    }
}

enum ClipboardHistoryPersistence {
    static func load(from url: URL) throws -> [ClipboardEntry] {
        let data = try Data(contentsOf: url)
        let decoder = JSONDecoder()
        if let snapshot = try? decoder.decode(ClipboardHistorySnapshot.self, from: data) {
            return backfillContentHashes(snapshot.entries)
        }
        let legacy = try decoder.decode([ClipboardEntry].self, from: data)
        return backfillContentHashes(legacy)
    }

    static func save(entries: [ClipboardEntry], to url: URL) {
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let snapshot = ClipboardHistorySnapshot(entries: entries)
        if let data = try? JSONEncoder().encode(snapshot) {
            try? data.write(to: url, options: .atomic)
        }
    }

    private static func backfillContentHashes(_ entries: [ClipboardEntry]) -> [ClipboardEntry] {
        entries.map { entry in
            guard entry.contentHash.isEmpty else { return entry }
            var updated = entry
            updated.contentHash = ClipboardContentHash.backfill(for: entry)
            return updated
        }
    }
}
