import Foundation
import EventKit

public struct CalendarEventSnapshot: Sendable, Hashable, Identifiable {
    public let id: String
    public let title: String
    public let startDate: Date
    public let endDate: Date
    public let calendarTitle: String

    public init(id: String, title: String, startDate: Date, endDate: Date, calendarTitle: String) {
        self.id = id
        self.title = title
        self.startDate = startDate
        self.endDate = endDate
        self.calendarTitle = calendarTitle
    }
}

public enum CalendarServiceError: Error, Equatable, Sendable {
    case accessDenied
    case noDefaultCalendar
    case saveFailed(message: String)
}

/// EventKit pass-through for Calendar events (ADR extension for `e` trigger).
public actor CalendarService {
    private let store: EKEventStore

    public init(store: EKEventStore = EKEventStore()) {
        self.store = store
    }

    public func authorization() -> RemindersAuthorization {
        let status = EKEventStore.authorizationStatus(for: .event)
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
                let granted = try await store.requestFullAccessToEvents()
                return granted ? .authorized : .denied
            } catch {
                return .denied
            }
        } else {
            return await withCheckedContinuation { continuation in
                store.requestAccess(to: .event) { granted, _ in
                    continuation.resume(returning: granted ? .authorized : .denied)
                }
            }
        }
    }

    public func todayEvents(now: Date = Date(), limit: Int = 8) async throws -> [CalendarEventSnapshot] {
        try await ensureAuthorized()
        let start = Calendar.current.startOfDay(for: now)
        let end = start.addingTimeInterval(86400)
        let predicate = store.predicateForEvents(withStart: start, end: end, calendars: nil)
        let events = store.events(matching: predicate)
            .sorted { $0.startDate < $1.startDate }
        return Array(events.prefix(limit)).map(Self.snapshot(from:))
    }

    @discardableResult
    public func create(title: String, startDate: Date, endDate: Date, notes: String? = nil) async throws -> CalendarEventSnapshot {
        try await ensureAuthorized()
        guard let calendar = store.defaultCalendarForNewEvents else {
            throw CalendarServiceError.noDefaultCalendar
        }
        let event = EKEvent(eventStore: store)
        event.title = title
        event.startDate = startDate
        event.endDate = endDate
        event.notes = notes
        event.calendar = calendar
        do {
            try store.save(event, span: .thisEvent, commit: true)
        } catch {
            throw CalendarServiceError.saveFailed(message: error.localizedDescription)
        }
        return Self.snapshot(from: event)
    }

    private func ensureAuthorized() async throws {
        switch authorization() {
        case .authorized:
            return
        case .notDetermined:
            let result = await requestAccess()
            if result != .authorized {
                throw CalendarServiceError.accessDenied
            }
        case .denied:
            throw CalendarServiceError.accessDenied
        }
    }

    private static func snapshot(from event: EKEvent) -> CalendarEventSnapshot {
        CalendarEventSnapshot(
            id: event.eventIdentifier ?? UUID().uuidString,
            title: event.title ?? "",
            startDate: event.startDate,
            endDate: event.endDate,
            calendarTitle: event.calendar?.title ?? ""
        )
    }
}
