import Foundation

public enum WordFamiliarity: String, Sendable, Codable {
    case known
    case unknown
    case mastered
    /// Retained for legacy DB rows; UI routes to `.known` scheduling.
    case fuzzy
}

public enum ReviewScheduler {
    public static let intervals: [Duration] = [
        .seconds(5 * 60),
        .seconds(30 * 60),
        .seconds(12 * 60 * 60),
        .seconds(1 * 24 * 60 * 60),
        .seconds(2 * 24 * 60 * 60),
        .seconds(4 * 24 * 60 * 60),
        .seconds(7 * 24 * 60 * 60),
        .seconds(15 * 24 * 60 * 60),
        .seconds(30 * 24 * 60 * 60)
    ]

    public static func schedule(familiarity: WordFamiliarity, currentStage: Int, wrongCount: Int) -> (stage: Int, delay: Duration) {
        switch familiarity {
        case .known:
            let intervalIndex = min(currentStage, intervals.count - 1)
            let stage = min(currentStage + 1, intervals.count)
            return (stage, intervals[intervalIndex])
        case .mastered:
            return (intervals.count, .seconds(60 * 60 * 24 * 365 * 100))
        case .fuzzy:
            return Self.schedule(familiarity: .known, currentStage: currentStage, wrongCount: wrongCount)
        case .unknown:
            let delay = wrongCount <= 1 ? intervals[0] : intervals[1]
            return (0, delay)
        }
    }
}
