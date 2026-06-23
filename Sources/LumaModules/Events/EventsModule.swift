import AppKit
import Foundation
import LumaCore
import LumaServices

public actor EventsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .events,
        displayName: "Events",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false,
        priority: 4,
        queryTimeout: .milliseconds(120)
    )

    private let calendar: CalendarService

    public init(calendar: CalendarService = CalendarService()) {
        self.calendar = calendar
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if ModuleHelp.isHelpQuery(trimmed) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        if trimmed.isEmpty {
            return await listToday()
        }

        let parsed = TodoTimeParser.parse(trimmed)
        let title = parsed.title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return ModuleResult(items: []) }

        let start = parsed.dueDate ?? Date().addingTimeInterval(3600)
        let end = start.addingTimeInterval(3600)
        return ModuleResult(items: [captureRow(title: title, start: start, end: end, original: trimmed)])
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(EventsAction.self, from: payload)
        switch decoded {
        case .create(let title, let start, let end):
            _ = try await calendar.create(title: title, startDate: Date(timeIntervalSince1970: start), endDate: Date(timeIntervalSince1970: end))
        case .grant:
            await MainActor.run {
                if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars") {
                    NSWorkspace.shared.open(url)
                }
            }
        }
    }

    private func listToday() async -> ModuleResult {
        let authorization = await calendar.authorization()
        if authorization == .denied {
            return ModuleResult(items: [permissionRow()])
        }
        do {
            let events = try await calendar.todayEvents()
            if events.isEmpty {
                return ModuleResult(items: [hintRow("No events today — try e meet john tomorrow 14:00")])
            }
            return ModuleResult(items: events.map(eventRow))
        } catch CalendarServiceError.accessDenied {
            return ModuleResult(items: [permissionRow()])
        } catch {
            return ModuleResult(items: [hintRow("Calendar unavailable")])
        }
    }

    private func captureRow(title: String, start: Date, end: Date, original: String) -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(EventsAction.create(
            title: title,
            start: start.timeIntervalSince1970,
            end: end.timeIntervalSince1970
        ))) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "capture:\(original)"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: "Create event · \(Self.formatEventTime(start))",
            icon: .symbol("calendar.badge.plus"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "create"),
                title: "Create Event",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func eventRow(_ event: CalendarEventSnapshot) -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: event.id),
            title: event.title,
            titleAttributed: AttributedString(event.title),
            subtitle: "\(Self.formatEventTime(event.startDate)) · \(event.calendarTitle)",
            icon: .symbol("calendar"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(event.id)"),
                title: "Open Calendar",
                kind: .openURL(URL(string: "x-apple-calendar://")!)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func hintRow(_ text: String) -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "hint:\(text)"),
            title: text,
            titleAttributed: AttributedString(text),
            subtitle: "Events",
            icon: .symbol("calendar"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "hint"),
                title: text,
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func permissionRow() -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(EventsAction.grant)) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "grant"),
            title: "Grant Calendar Access",
            titleAttributed: AttributedString("Grant Calendar Access"),
            subtitle: "Open System Settings to allow Luma",
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "grant"),
                title: "Open Settings",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "e" || lower == "event" {
            return ""
        }
        if lower.hasPrefix("e ") {
            return String(trimmed.dropFirst(2))
        }
        if lower.hasPrefix("event ") {
            return String(trimmed.dropFirst(6))
        }
        return nil
    }

    private static func formatEventTime(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}
