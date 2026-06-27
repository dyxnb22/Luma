import Foundation
import LumaCore
import LumaServices

public enum MenuItemsIndex {
    public static func search(_ records: [MenuItemRecord], query: String, limit: Int = 8) -> [MenuItemMatch] {
        let q = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !q.isEmpty else { return Array(records.prefix(limit)).map { MenuItemMatch(record: $0, score: 0.2) } }
        return records.compactMap { record -> MenuItemMatch? in
            guard let leaf = record.titlePath.last?.lowercased() else { return nil }
            let prefix = record.titlePath.dropLast().joined(separator: " ").lowercased()
            let leafScore = FuzzyMatcher.score(query: q, target: leaf)
            let pathScore = FuzzyMatcher.score(query: q, target: prefix)
            let score = leafScore * 0.8 + pathScore * 0.2
            guard score > 0 else { return nil }
            return MenuItemMatch(record: record, score: score)
        }
        .sorted { $0.score > $1.score }
        .prefix(limit)
        .map { $0 }
    }
}
