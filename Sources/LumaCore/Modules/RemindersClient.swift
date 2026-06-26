import Foundation

public protocol RemindersClient: Sendable {
    func authorization() async -> RemindersAuthorization
    func requestAccess() async -> RemindersAuthorization
    func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot]
    func noDueDate(limit: Int) async throws -> [ReminderSnapshot]
    func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot]
    func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot]
    func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot
    func complete(id: String) async throws
    func uncomplete(id: String) async throws
    func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot
}

public struct NoopRemindersClient: RemindersClient {
    public init() {}

    public func authorization() async -> RemindersAuthorization { .notDetermined }
    public func requestAccess() async -> RemindersAuthorization { .notDetermined }

    public func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        _ = now
        _ = limit
        return []
    }

    public func noDueDate(limit: Int) async throws -> [ReminderSnapshot] {
        _ = limit
        return []
    }

    public func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        _ = now
        _ = limit
        return []
    }

    public func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        _ = now
        _ = limit
        return []
    }

    public func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot {
        _ = title
        _ = dueDate
        _ = notes
        throw RemindersServiceError.accessDenied
    }

    public func complete(id: String) async throws {
        _ = id
        throw RemindersServiceError.accessDenied
    }

    public func uncomplete(id: String) async throws {
        _ = id
        throw RemindersServiceError.accessDenied
    }

    public func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot {
        _ = id
        _ = title
        _ = dueDate
        _ = clearDueDate
        throw RemindersServiceError.accessDenied
    }
}
