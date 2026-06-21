import Foundation

public enum Ranker {
    public static func score(item: ResultItem, query: Query, usage: UsageRecord?, now: Date = Date()) -> Double {
        let fuzzy = query.normalized.isEmpty ? 1.0 : FuzzyMatcher.score(query: query.normalized, target: item.title.lowercased())
        if !query.normalized.isEmpty && fuzzy <= 0 {
            return -.infinity
        }

        let recency: Double
        if let lastUsed = usage?.lastUsed {
            let days = max(0, now.timeIntervalSince(lastUsed) / 86_400)
            recency = exp(-days / 7.0)
        } else {
            recency = 0
        }

        let frequency: Double
        if let usage {
            frequency = log1p(Double(usage.count)) / log1p(50.0)
        } else {
            frequency = 0
        }

        let modulePriority = Double(item.rankingHints.basePriority) / 10.0

        return 0.55 * fuzzy
            + 0.20 * recency
            + 0.15 * frequency
            + 0.10 * modulePriority
    }
}
