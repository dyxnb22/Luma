import Foundation

public enum MediaParser {
    public enum Mode: Sendable, Equatable {
        case capture(partial: Bool)
        case search
    }

    public struct Result: Sendable, Equatable {
        public let mode: Mode
        public let title: String
        public let category: MediaCategory?
        public let rating: Int?
        public let status: MediaStatus
        public let tags: [String]
        public let hadDSLToken: Bool
    }

    private static let categoryTokens: [String: MediaCategory] = [
        "movie": .movie, "film": .movie, "电影": .movie,
        "tv": .tv, "show": .tv, "series": .tv, "剧": .tv, "电视剧": .tv,
        "anime": .anime, "番": .anime, "动漫": .anime,
        "game": .game, "游戏": .game,
        "book": .book, "novel": .book, "书": .book, "小说": .book
    ]

    private static let statusTokens: [String: MediaStatus] = [
        "planning": .planned, "plan": .planned, "planned": .planned,
        "想看": .planned, "想读": .planned, "想玩": .planned,
        "watching": .inProgress, "reading": .inProgress, "playing": .inProgress, "wip": .inProgress,
        "在看": .inProgress, "在读": .inProgress, "在玩": .inProgress,
        "done": .done, "finished": .done, "complete": .done, "completed": .done,
        "看完": .done, "读完": .done, "通关": .done, "完成": .done,
        "abandoned": .abandoned, "dropped": .abandoned, "弃": .abandoned, "弃坑": .abandoned
    ]

    public static func parse(_ raw: String) -> Result {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return Result(mode: .search, title: "", category: nil, rating: nil, status: .done, tags: [], hadDSLToken: false)
        }

        var tokens = trimmed.split(separator: " ").map(String.init)
        let tags = extractTags(from: &tokens)
        var category: MediaCategory?
        var rating: Int?
        var status: MediaStatus = .done
        var hadDSLToken = false

        while !tokens.isEmpty {
            let token = tokens[tokens.count - 1]
            let lowered = token.lowercased()

            if let mapped = categoryTokens[lowered] ?? categoryTokens[token] {
                category = mapped
                hadDSLToken = true
                tokens.removeLast()
                continue
            }

            if let mapped = statusTokens[lowered] ?? statusTokens[token] {
                status = mapped
                hadDSLToken = true
                tokens.removeLast()
                continue
            }

            if let parsedRating = parseRating(token, isFirstToken: tokens.count == 1) {
                rating = parsedRating
                hadDSLToken = true
                tokens.removeLast()
                continue
            }

            break
        }

        let title = tokens.joined(separator: " ").trimmingCharacters(in: .whitespacesAndNewlines)

        if hadDSLToken {
            let partial = category == nil
            return Result(
                mode: .capture(partial: partial),
                title: title,
                category: category,
                rating: rating,
                status: status,
                tags: tags,
                hadDSLToken: true
            )
        }

        return Result(mode: .search, title: title, category: nil, rating: nil, status: .done, tags: tags, hadDSLToken: false)
    }

    private static func extractTags(from tokens: inout [String]) -> [String] {
        var tags: [String] = []
        tokens = tokens.compactMap { token in
            guard token.hasPrefix("#") else { return token }
            let tag = String(token.dropFirst()).lowercased().trimmingCharacters(in: .whitespacesAndNewlines)
            if !tag.isEmpty { tags.append(tag) }
            return nil
        }
        return tags
    }

    private static func parseRating(_ token: String, isFirstToken: Bool) -> Int? {
        var valueText = token

        if valueText.lowercased().hasPrefix("rating:") {
            valueText = String(valueText.dropFirst("rating:".count))
        } else if valueText.count >= 2,
                  valueText.first?.lowercased() == "r",
                  valueText.dropFirst().allSatisfy(\.isNumber) {
            valueText = String(valueText.dropFirst())
        } else if let starIndex = valueText.firstIndex(where: { $0 == "★" || $0 == "☆" }) {
            valueText = String(valueText[starIndex...].drop(while: { $0 == "★" || $0 == "☆" }))
        }

        if valueText.contains("/") {
            let parts = valueText.split(separator: "/")
            guard parts.count == 2, let value = Int(parts[0]), (1...10).contains(value) else { return nil }
            return value
        }
        guard let value = Int(valueText), (1...10).contains(value) else { return nil }
        if isFirstToken { return nil }
        return value
    }
}
