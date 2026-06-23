import Foundation

public struct WordbookProgressSnapshot: Sendable {
    public let total: Int
    public let mastered: Int
    public let dueToday: Int
    public let newAvailable: Int
    public let todayReviewed: Int
    public let todayNewLearned: Int
    public let accuracyToday: Double
    public let streakDays: Int
    public let dailyNewLimit: Int
    public let dailyNewSeen: Int

    public init(
        total: Int,
        mastered: Int,
        dueToday: Int,
        newAvailable: Int,
        todayReviewed: Int,
        todayNewLearned: Int,
        accuracyToday: Double,
        streakDays: Int,
        dailyNewLimit: Int,
        dailyNewSeen: Int
    ) {
        self.total = total
        self.mastered = mastered
        self.dueToday = dueToday
        self.newAvailable = newAvailable
        self.todayReviewed = todayReviewed
        self.todayNewLearned = todayNewLearned
        self.accuracyToday = accuracyToday
        self.streakDays = streakDays
        self.dailyNewLimit = dailyNewLimit
        self.dailyNewSeen = dailyNewSeen
    }
}

public enum WordbookCSVImportResult: Sendable {
    case imported(count: Int, skipped: Int)
}

public enum WordbookDateFormat {
    public static func iso(_ date: Date) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.string(from: date)
    }
}
