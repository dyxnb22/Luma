import Foundation

public struct ReminderSnapshot: Sendable, Hashable, Identifiable {
    public let id: String
    public let title: String
    public let dueDate: Date?
    public let isCompleted: Bool
    public let calendarTitle: String
    public let creationDate: Date?
    public let completionDate: Date?

    public init(
        id: String,
        title: String,
        dueDate: Date?,
        isCompleted: Bool,
        calendarTitle: String,
        creationDate: Date? = nil,
        completionDate: Date? = nil
    ) {
        self.id = id
        self.title = title
        self.dueDate = dueDate
        self.isCompleted = isCompleted
        self.calendarTitle = calendarTitle
        self.creationDate = creationDate
        self.completionDate = completionDate
    }
}

public enum RemindersServiceError: Error, Equatable, Sendable {
    case accessDenied
    case noDefaultCalendar
    case notFound(id: String)
    case saveFailed(message: String)
}

public enum RemindersAuthorization: Sendable {
    case authorized
    case denied
    case notDetermined
}
