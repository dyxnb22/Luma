import Foundation

/// Narrow clients for continue-flow home rows without coupling to concrete module types.
public protocol NotesContinueClient: Sendable {
    func dailyNotePath() async -> String?
}

public protocol TodoContinueClient: Sendable {
    func firstTodayDueReminder() async throws -> ReminderSnapshot?
}

public protocol MediaContinueClient: Sendable {
    func inProgressCount() async -> Int
}

public protocol WordbookContinueClient: Sendable {
    func dueTodayCount() async -> Int
}
