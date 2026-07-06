import Foundation

public enum Ranker {
    public struct RankedScore: Sendable {
        public let finalScore: Double
        public let fuzzyScore: Double
    }

    public static func score(item: ResultItem, query: Query, usage: UsageRecord?, now: Date = Date()) -> RankedScore {
        let fuzzy = fuzzyScore(
            query: query,
            target: item.title.lowercased(),
            secondary: item.subtitle?.lowercased()
        )
        if !fuzzyMatchingText(for: query).isEmpty && fuzzy <= 0 {
            return RankedScore(finalScore: -.infinity, fuzzyScore: fuzzy)
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

        let matchText = fuzzyMatchingText(for: query)
        let exactBoost: Double = (!matchText.isEmpty && item.title.lowercased() == matchText) ? 0.3 : 0.0

        let finalScore = 0.45 * fuzzy
            + 0.20 * recency
            + 0.15 * frequency
            + 0.10 * modulePriority
            + exactBoost
        return RankedScore(finalScore: finalScore, fuzzyScore: fuzzy)
    }

    static func fuzzyMatchingText(for query: Query) -> String {
        if let command = query.command {
            let payload = command.payload.trimmingCharacters(in: .whitespacesAndNewlines)
            if !payload.isEmpty {
                return payload.lowercased()
            }
            // Empty payload is a module home view — do not filter rows by the trigger token.
            return ""
        }
        return query.normalized
    }

    static func fuzzyScore(query: Query, target: String, secondary: String? = nil) -> Double {
        let text = fuzzyMatchingText(for: query)
        guard !text.isEmpty else { return 1.0 }

        let direct = FuzzyMatcher.score(query: text, target: target)
        if direct > 0 { return direct }

        if let secondary, !secondary.isEmpty {
            let secondaryScore = FuzzyMatcher.score(query: text, target: secondary)
            if secondaryScore > 0 { return secondaryScore }
        }

        let tokens = text.split(separator: " ")
        guard tokens.count > 1 else { return direct }

        return tokens
            .map { token in
                max(
                    FuzzyMatcher.score(query: String(token), target: target),
                    secondary.map { FuzzyMatcher.score(query: String(token), target: $0) } ?? 0
                )
            }
            .max() ?? 0
    }
}
