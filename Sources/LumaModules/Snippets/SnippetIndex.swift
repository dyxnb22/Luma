import Foundation
import LumaCore

public struct SnippetSearchResult: Sendable, Hashable {
    public let snippet: Snippet
    public let score: Double
}

public enum SnippetIndex {
    public static func topByFrecency(_ snippets: [Snippet], limit: Int = 8, now: Date = Date()) -> [Snippet] {
        snippets
            .map { snippet -> (Snippet, Double) in
                (snippet, frecencyScore(fuzzyMatch: 1.0, snippet: snippet, now: now))
            }
            .sorted { lhs, rhs in
                if lhs.1 != rhs.1 { return lhs.1 > rhs.1 }
                return lhs.0.title.localizedStandardCompare(rhs.0.title) == .orderedAscending
            }
            .prefix(limit)
            .map(\.0)
    }

    public static func search(_ snippets: [Snippet], query: String, limit: Int = 8, now: Date = Date()) -> [SnippetSearchResult] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return topByFrecency(snippets, limit: limit, now: now).map {
                SnippetSearchResult(snippet: $0, score: 1.0)
            }
        }

        let lowered = trimmed.lowercased()
        var results: [SnippetSearchResult] = []

        for snippet in snippets {
            let titleScore = FuzzyMatcher.score(query: lowered, target: snippet.title.lowercased())
            let contentScore = snippet.content.lowercased().hasPrefix(lowered)
                ? 0.85
                : FuzzyMatcher.score(query: lowered, target: String(snippet.content.prefix(120)).lowercased())
            var fuzzyMatch = max(titleScore, contentScore * 0.9)
            if snippet.tags.contains(where: { $0 == lowered || $0.hasPrefix(lowered) }) {
                fuzzyMatch = min(1.0, fuzzyMatch + 0.15)
            }
            guard fuzzyMatch > 0 else { continue }
            let score = frecencyScore(fuzzyMatch: fuzzyMatch, snippet: snippet, now: now)
            results.append(SnippetSearchResult(snippet: snippet, score: score))
        }

        return results
            .sorted { lhs, rhs in
                if lhs.score != rhs.score { return lhs.score > rhs.score }
                return lhs.snippet.title.localizedStandardCompare(rhs.snippet.title) == .orderedAscending
            }
            .prefix(limit)
            .map { $0 }
    }

    static func frecencyScore(fuzzyMatch: Double, snippet: Snippet, now: Date) -> Double {
        let usageComponent = log(1.0 + Double(snippet.usageCount)) / log(51.0)
        let ageSeconds = now.timeIntervalSince(snippet.lastUsedAt)
        let recencyComponent = exp(-ageSeconds / 86400.0)
        return fuzzyMatch * 0.5 + usageComponent * 0.3 + recencyComponent * 0.2
    }
}
