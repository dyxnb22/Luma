import Foundation
import LumaCore

public struct MediaSearchResult: Sendable, Hashable {
    public let item: MediaItem
    public let score: Double
}

public enum MediaSort: String, Sendable, CaseIterable {
    case recentlyCompleted
    case recentlyAdded
    case ratingDesc
    case title
}

public enum MediaIndex {
    public static func search(_ items: [MediaItem], query: String, limit: Int = 8, now: Date = Date()) -> [MediaSearchResult] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return recent(items, limit: limit).map { MediaSearchResult(item: $0, score: 1.0) }
        }

        let lowered = trimmed.lowercased()
        var results: [MediaSearchResult] = []
        for item in items {
            let fuzzy = FuzzyMatcher.score(query: lowered, target: item.title.lowercased())
            guard fuzzy > 0 else { continue }
            let recency = recencyBoost(for: item, now: now)
            let score = fuzzy * 0.6 + recency * 0.4
            results.append(MediaSearchResult(item: item, score: score))
        }

        return results
            .sorted { lhs, rhs in
                if lhs.score != rhs.score { return lhs.score > rhs.score }
                return lhs.item.title.localizedStandardCompare(rhs.item.title) == .orderedAscending
            }
            .prefix(limit)
            .map { $0 }
    }

    public static func recent(_ items: [MediaItem], limit: Int = 8) -> [MediaItem] {
        items
            .sorted { $0.updatedAt > $1.updatedAt }
            .prefix(limit)
            .map { $0 }
    }

    public static func filter(
        _ items: [MediaItem],
        category: MediaCategory?,
        status: MediaStatus?,
        sort: MediaSort
    ) -> [MediaItem] {
        var filtered = items
        if let category {
            filtered = filtered.filter { $0.category == category }
        }
        if let status {
            filtered = filtered.filter { $0.status == status }
        }
        switch sort {
        case .recentlyCompleted:
            filtered.sort {
                let l = $0.completedAt ?? $0.updatedAt
                let r = $1.completedAt ?? $1.updatedAt
                if l != r { return l > r }
                return $0.title.localizedStandardCompare($1.title) == .orderedAscending
            }
        case .recentlyAdded:
            filtered.sort {
                if $0.createdAt != $1.createdAt { return $0.createdAt > $1.createdAt }
                return $0.title.localizedStandardCompare($1.title) == .orderedAscending
            }
        case .ratingDesc:
            filtered.sort {
                let lr = $0.rating ?? -1
                let rr = $1.rating ?? -1
                if lr != rr { return lr > rr }
                return $0.title.localizedStandardCompare($1.title) == .orderedAscending
            }
        case .title:
            filtered.sort { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
        }
        return filtered
    }

    public static func stats(for items: [MediaItem], calendar: Calendar = .current) -> (count: Int, averageRating: Double?, doneThisYear: Int) {
        let rated = items.compactMap(\.rating)
        let average = rated.isEmpty ? nil : Double(rated.reduce(0, +)) / Double(rated.count)
        let year = calendar.component(.year, from: Date())
        let doneThisYear = items.filter { item in
            guard item.status == .done, let completed = item.completedAt else { return false }
            return calendar.component(.year, from: completed) == year
        }.count
        return (items.count, average, doneThisYear)
    }

    private static func recencyBoost(for item: MediaItem, now: Date) -> Double {
        let age = now.timeIntervalSince(item.updatedAt)
        return exp(-age / (86400.0 * 30.0))
    }
}
