import Foundation
import EventKit

public struct ReminderSnapshot: Sendable, Hashable, Identifiable {
    public let id: String
    public let title: String
    public let dueDate: Date?
    public let isCompleted: Bool
    public let calendarTitle: String

    public init(id: String, title: String, dueDate: Date?, isCompleted: Bool, calendarTitle: String) {
        self.id = id
        self.title = title
        self.dueDate = dueDate
        self.isCompleted = isCompleted
        self.calendarTitle = calendarTitle
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

/// Thin EventKit pass-through used by the Todo module.
///
/// Ownership boundary: this service never holds reminder data of its own. Reminders.app remains the
/// source of truth; Luma only requests access, reads today's due items, creates new items, and marks
/// existing items complete. See ADR-009.
public actor RemindersService {
    private let store: EKEventStore

    public init(store: EKEventStore = EKEventStore()) {
        self.store = store
    }

    public func authorization() -> RemindersAuthorization {
        let status: EKAuthorizationStatus
        if #available(macOS 14.0, *) {
            status = EKEventStore.authorizationStatus(for: .reminder)
        } else {
            status = EKEventStore.authorizationStatus(for: .reminder)
        }
        switch status {
        case .authorized, .fullAccess:
            return .authorized
        case .denied, .restricted:
            return .denied
        case .notDetermined, .writeOnly:
            return .notDetermined
        @unknown default:
            return .notDetermined
        }
    }

    public func requestAccess() async -> RemindersAuthorization {
        if #available(macOS 14.0, *) {
            do {
                let granted = try await store.requestFullAccessToReminders()
                return granted ? .authorized : .denied
            } catch {
                return .denied
            }
        } else {
            return await withCheckedContinuation { continuation in
                store.requestAccess(to: .reminder) { granted, _ in
                    continuation.resume(returning: granted ? .authorized : .denied)
                }
            }
        }
    }

    /// Reminders due today (or overdue), incomplete, ordered by due date ascending.
    public func todayDue(now: Date = Date(), limit: Int = 8) async throws -> [ReminderSnapshot] {
        try await ensureAuthorized()
        let calendars = store.calendars(for: .reminder)
        let endOfToday = Calendar.current.startOfDay(for: now).addingTimeInterval(60 * 60 * 24)
        let predicate = store.predicateForIncompleteReminders(
            withDueDateStarting: nil,
            ending: endOfToday,
            calendars: calendars
        )
        let snapshots = try await fetchSnapshots(predicate: predicate)
        let sorted = snapshots.sorted { lhs, rhs in
            let lDate = lhs.dueDate ?? .distantFuture
            let rDate = rhs.dueDate ?? .distantFuture
            return lDate < rDate
        }
        return Array(sorted.prefix(limit))
    }

    /// Reminders without a due date, useful for the Todo "no time set" list.
    public func upcoming(limit: Int = 8) async throws -> [ReminderSnapshot] {
        try await ensureAuthorized()
        let calendars = store.calendars(for: .reminder)
        let predicate = store.predicateForIncompleteReminders(
            withDueDateStarting: nil,
            ending: nil,
            calendars: calendars
        )
        let snapshots = try await fetchSnapshots(predicate: predicate)
        return Array(snapshots.prefix(limit))
    }

    /// Reminders due after today, incomplete.
    public func futureDue(now: Date = Date(), limit: Int = 20) async throws -> [ReminderSnapshot] {
        try await ensureAuthorized()
        let calendars = store.calendars(for: .reminder)
        let startOfTomorrow = Calendar.current.startOfDay(for: now).addingTimeInterval(60 * 60 * 24)
        let predicate = store.predicateForIncompleteReminders(
            withDueDateStarting: startOfTomorrow,
            ending: nil,
            calendars: calendars
        )
        let snapshots = try await fetchSnapshots(predicate: predicate)
        let sorted = snapshots.sorted { ($0.dueDate ?? .distantFuture) < ($1.dueDate ?? .distantFuture) }
        return Array(sorted.prefix(limit))
    }

    /// Recently completed reminders (last 7 days).
    public func completedRecently(now: Date = Date(), limit: Int = 20) async throws -> [ReminderSnapshot] {
        try await ensureAuthorized()
        let calendars = store.calendars(for: .reminder)
        let weekAgo = Calendar.current.date(byAdding: .day, value: -7, to: now) ?? now
        let predicate = store.predicateForCompletedReminders(
            withCompletionDateStarting: weekAgo,
            ending: now,
            calendars: calendars
        )
        let snapshots = try await fetchSnapshots(predicate: predicate)
        let sorted = snapshots.sorted { ($0.dueDate ?? now) > ($1.dueDate ?? now) }
        return Array(sorted.prefix(limit))
    }

    @discardableResult
    public func create(title: String, dueDate: Date? = nil, notes: String? = nil) async throws -> ReminderSnapshot {
        try await ensureAuthorized()
        guard let calendar = store.defaultCalendarForNewReminders() else {
            throw RemindersServiceError.noDefaultCalendar
        }
        let reminder = EKReminder(eventStore: store)
        reminder.title = title
        reminder.notes = notes
        reminder.calendar = calendar
        if let dueDate {
            reminder.dueDateComponents = Calendar.current.dateComponents(
                [.year, .month, .day, .hour, .minute],
                from: dueDate
            )
        }
        do {
            try store.save(reminder, commit: true)
        } catch {
            throw RemindersServiceError.saveFailed(message: error.localizedDescription)
        }
        return Self.snapshot(from: reminder)
    }

    public func complete(id: String) async throws {
        try await ensureAuthorized()
        guard let reminder = store.calendarItem(withIdentifier: id) as? EKReminder else {
            throw RemindersServiceError.notFound(id: id)
        }
        reminder.isCompleted = true
        do {
            try store.save(reminder, commit: true)
        } catch {
            throw RemindersServiceError.saveFailed(message: error.localizedDescription)
        }
    }

    public func update(id: String, title: String, dueDate: Date?) async throws -> ReminderSnapshot {
        try await ensureAuthorized()
        guard let reminder = store.calendarItem(withIdentifier: id) as? EKReminder else {
            throw RemindersServiceError.notFound(id: id)
        }
        reminder.title = title
        if let dueDate {
            reminder.dueDateComponents = Calendar.current.dateComponents(
                [.year, .month, .day, .hour, .minute],
                from: dueDate
            )
        } else {
            reminder.dueDateComponents = nil
        }
        do {
            try store.save(reminder, commit: true)
        } catch {
            throw RemindersServiceError.saveFailed(message: error.localizedDescription)
        }
        return Self.snapshot(from: reminder)
    }

    private func ensureAuthorized() async throws {
        switch authorization() {
        case .authorized:
            return
        case .notDetermined:
            let result = await requestAccess()
            if result != .authorized {
                throw RemindersServiceError.accessDenied
            }
        case .denied:
            throw RemindersServiceError.accessDenied
        }
    }

    private func fetchSnapshots(predicate: NSPredicate) async throws -> [ReminderSnapshot] {
        try await withCheckedThrowingContinuation { continuation in
            store.fetchReminders(matching: predicate) { reminders in
                let snapshots = (reminders ?? []).map(Self.snapshot(from:))
                continuation.resume(returning: snapshots)
            }
        }
    }

    private static func snapshot(from reminder: EKReminder) -> ReminderSnapshot {
        let due = reminder.dueDateComponents.flatMap { Calendar.current.date(from: $0) }
        return ReminderSnapshot(
            id: reminder.calendarItemIdentifier,
            title: reminder.title ?? "",
            dueDate: due,
            isCompleted: reminder.isCompleted,
            calendarTitle: reminder.calendar?.title ?? ""
        )
    }
}
