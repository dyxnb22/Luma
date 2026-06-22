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
        public let hadDSLToken: Bool
    }

    private static let categoryTokens: [String: MediaCategory] = [
        "movie": .movie, "film": .movie,
        "tv": .tv, "show": .tv, "series": .tv,
        "anime": .anime,
        "game": .game,
        "book": .book, "novel": .book
    ]

    private static let statusTokens: [String: MediaStatus] = [
        "planning": .planned, "plan": .planned,
        "watching": .inProgress, "reading": .inProgress, "playing": .inProgress, "wip": .inProgress,
        "done": .done, "finished": .done,
        "abandoned": .abandoned, "dropped": .abandoned
    ]

    public static func parse(_ raw: String) -> Result {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return Result(mode: .search, title: "", category: nil, rating: nil, status: .done, hadDSLToken: false)
        }

        var tokens = trimmed.split(separator: " ").map(String.init)
        var category: MediaCategory?
        var rating: Int?
        var status: MediaStatus = .done
        var hadDSLToken = false

        while !tokens.isEmpty {
            let token = tokens[tokens.count - 1]
            let lowered = token.lowercased()

            if let mapped = categoryTokens[lowered] {
                category = mapped
                hadDSLToken = true
                tokens.removeLast()
                continue
            }

            if let mapped = statusTokens[lowered] {
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
            return Result(mode: .capture(partial: partial), title: title, category: category, rating: rating, status: status, hadDSLToken: true)
        }

        return Result(mode: .search, title: trimmed, category: nil, rating: nil, status: .done, hadDSLToken: false)
    }

    private static func parseRating(_ token: String, isFirstToken: Bool) -> Int? {
        if token.contains("/") {
            let parts = token.split(separator: "/")
            guard parts.count == 2, let value = Int(parts[0]), (1...10).contains(value) else { return nil }
            return value
        }
        guard let value = Int(token), (1...10).contains(value) else { return nil }
        if isFirstToken { return nil }
        return value
    }
}
