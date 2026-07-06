import Foundation
import LumaCore

public enum TodoChangeHub {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var continuations: [UUID: AsyncStream<Void>.Continuation] = [:]

    public static func dataChanges() -> AsyncStream<Void> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            continuations[id] = continuation
            lock.unlock()
            continuation.onTermination = { _ in
                lock.lock()
                continuations.removeValue(forKey: id)
                lock.unlock()
            }
        }
    }

    public static func publishDataChanged() {
        lock.lock()
        let targets = continuations.values
        lock.unlock()
        for continuation in targets {
            continuation.yield()
        }
    }
}

/// EventKit pass-through Todo module (see ADR-009).
///
/// Trigger: `t ` or `todo ` prefix.
///
/// - `t buy milk` -> create a reminder titled "buy milk" on the default Reminders list.
/// - `t pay rent tomorrow 9:00` -> create with due date tomorrow at 09:00.
/// - `t` or `todo` with no text -> list today's due reminders. Return on a row marks it complete.
///
/// Luma never persists TODO state of its own. Reminders.app remains source of truth.
public actor TodoModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .todo,
        displayName: "Todo",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(60)
    )

    private var reminders: any RemindersClient = NoopRemindersClient()
    private let now: @Sendable () -> Date
    private var cachedDue: [ReminderSnapshot]?
    private var cachedAt: Date?
    private var storeChangesTask: Task<Void, Never>?
    private var contentRevision: UInt64 = 0

    private static let dueListLimit = 8

    public init(now: @Sendable @escaping () -> Date = { Date() }) {
        self.now = now
    }

    public func detailContentRevision() async -> UInt64 {
        contentRevision
    }

    public func warmup(_ context: ModuleContext) async {
        reminders = context.platform.reminders
        if await reminders.authorization() == .authorized {
            _ = try? await refreshDueCache(force: true)
        }
        startStoreChangesListener()
    }

    public func teardown() async {
        storeChangesTask?.cancel()
        storeChangesTask = nil
    }

    public func todayDueCount() async throws -> Int {
        guard await reminders.authorization() == .authorized else { return 0 }
        return try await reminders.todayDue(now: now(), limit: 100).count
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let raw = query.raw
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: raw) else {
            return ModuleResult(items: [])
        }

        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)

        // Permission gate: surface a single result to grant access if not yet authorized.
        let authorization = await reminders.authorization()
        if authorization == .denied {
            return ModuleResult(items: [permissionRow(authorization: authorization)])
        }
        if authorization == .notDetermined, trimmed.isEmpty {
            return ModuleResult(items: [permissionRow(authorization: authorization)])
        }
        if ModuleHelp.isHelpQuery(trimmed) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if trimmed.isEmpty {
            // List today's due (cached to avoid EventKit IPC on every keystroke)
            do {
                let due = try await cachedTodayDue()
                var items = [openDetailRow(dueCount: due.count)]
                if due.isEmpty {
                    items.append(emptyTodayRow())
                } else {
                    items.append(contentsOf: due.map(dueRow(_:)))
                }
                return ModuleResult(items: items)
            } catch RemindersServiceError.accessDenied {
                return ModuleResult(items: [permissionRow(authorization: .denied)])
            } catch {
                return ModuleResult(items: [])
            }
        }

        // Capture path: preview the parsed reminder. Return creates it.
        let parsed = TodoTimeParser.parse(trimmed, now: now())
        return ModuleResult(items: [captureRow(parsed: parsed, original: trimmed)])
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(TodoAction.self, from: payload)

        switch decoded {
        case .create(let title):
            let parsed = TodoTimeParser.parse(title, now: now())
            _ = try await reminders.create(title: parsed.title.isEmpty ? title : parsed.title, dueDate: parsed.dueDate, notes: nil)
            invalidateDueCache()
        case .complete(let id):
            try await reminders.complete(id: id)
            invalidateDueCache()
        case .uncomplete(let id):
            try await reminders.uncomplete(id: id)
            invalidateDueCache()
        case .grant:
            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders") {
                await context.platform.workspace.openURL(url)
            }
        case .requestAccess:
            _ = await reminders.requestAccess()
            invalidateDueCache()
        }
    }

    public func authorizationStatus() async -> RemindersAuthorization {
        await reminders.authorization()
    }

    public func requestRemindersAccess() async -> RemindersAuthorization {
        await reminders.requestAccess()
    }

    public func reminders(kind: TodoListKind, limit: Int = 30) async throws -> [ReminderSnapshot] {
        switch kind {
        case .today:
            return try await todayDue(limit: limit)
        case .inbox:
            return try await noDueDate(limit: limit)
        case .upcoming:
            return try await futureDue(limit: limit)
        case .completed:
            return try await completedReminders(limit: limit)
        }
    }

    public func todayDue(limit: Int = 20) async throws -> [ReminderSnapshot] {
        try await reminders.todayDue(now: now(), limit: limit)
    }

    public func noDueDate(limit: Int = 20) async throws -> [ReminderSnapshot] {
        try await reminders.noDueDate(limit: limit)
    }

    public func futureDue(limit: Int = 20) async throws -> [ReminderSnapshot] {
        try await reminders.futureDue(now: now(), limit: limit)
    }

    public func completedReminders(limit: Int = 20) async throws -> [ReminderSnapshot] {
        try await reminders.completedRecently(now: now(), limit: limit)
    }

    @discardableResult
    public func createReminder(from rawTitle: String) async throws -> ReminderSnapshot {
        let parsed = TodoTimeParser.parse(rawTitle, now: now())
        let title = parsed.title.isEmpty ? rawTitle : parsed.title
        let snapshot = try await reminders.create(title: title, dueDate: parsed.dueDate, notes: nil)
        invalidateDueCache()
        return snapshot
    }

    public func completeReminder(id: String) async throws {
        try await reminders.complete(id: id)
        invalidateDueCache()
    }

    public func uncompleteReminder(id: String) async throws {
        try await reminders.uncomplete(id: id)
        invalidateDueCache()
    }

    @discardableResult
    public func updateReminder(id: String, rawTitle: String, existingDueDate: Date? = nil) async throws -> ReminderSnapshot {
        let parsed = TodoTimeParser.parse(rawTitle, now: now())
        let title = parsed.title.isEmpty ? rawTitle : parsed.title
        let snapshot = try await reminders.update(id: id, title: title, dueDate: parsed.dueDate ?? existingDueDate, clearDueDate: false)
        invalidateDueCache()
        return snapshot
    }

    @discardableResult
    public func clearDueDate(id: String, title: String) async throws -> ReminderSnapshot {
        let snapshot = try await reminders.update(id: id, title: title, dueDate: nil, clearDueDate: true)
        invalidateDueCache()
        return snapshot
    }

    @discardableResult
    public func scheduleReminder(id: String, title: String, dueDate: Date) async throws -> ReminderSnapshot {
        let snapshot = try await reminders.update(id: id, title: title, dueDate: dueDate, clearDueDate: false)
        invalidateDueCache()
        return snapshot
    }

    public func firstTodayDueReminder() async throws -> ReminderSnapshot? {
        try await cachedTodayDue().first
    }

    // MARK: - Due cache

    private func cachedTodayDue() async throws -> [ReminderSnapshot] {
        let currentNow = now()
        if let cachedDue, let cachedAt, currentNow.timeIntervalSince(cachedAt) < CacheTTL.dueListSeconds {
            return cachedDue
        }
        return try await refreshDueCache(force: false)
    }

    @discardableResult
    private func refreshDueCache(force: Bool) async throws -> [ReminderSnapshot] {
        let currentNow = now()
        if !force, let cachedDue, let cachedAt, currentNow.timeIntervalSince(cachedAt) < CacheTTL.dueListSeconds {
            return cachedDue
        }
        let due = try await reminders.todayDue(now: currentNow, limit: Self.dueListLimit)
        cachedDue = due
        cachedAt = currentNow
        return due
    }

    private func invalidateDueCache() {
        cachedDue = nil
        cachedAt = nil
        contentRevision &+= 1
        TodoChangeHub.publishDataChanged()
    }

    private func startStoreChangesListener() {
        storeChangesTask?.cancel()
        storeChangesTask = Task { await observeStoreChanges() }
    }

    private func observeStoreChanges() async {
        let stream = await reminders.storeChanges()
        for await _ in stream {
            if Task.isCancelled { break }
            invalidateDueCache()
        }
    }

    // MARK: - Trigger parsing

    /// Returns the text after the `t ` / `todo ` prefix, or nil if the query does not target Todo.
    /// Empty string is valid (used to list today's due).
    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "t" || lower == "todo" {
            return ""
        }
        if lower.hasPrefix("t ") {
            return String(trimmed.dropFirst(2))
        }
        if lower.hasPrefix("todo ") {
            return String(trimmed.dropFirst(5))
        }
        return nil
    }

    /// Builds a launcher query that restores a todo capture without duplicating command prefixes.
    public static func resumeQuery(forCapture capture: String) -> String {
        let trimmed = capture.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "todo" }
        if extractPayload(raw: trimmed) != nil {
            return trimmed
        }
        return "todo \(trimmed)"
    }

    // MARK: - Result rows

    private func captureRow(parsed: TodoTimeParser.Parsed, original: String) -> ResultItem {
        let titleText = parsed.title.isEmpty ? original : parsed.title
        let subtitle: String
        if let due = parsed.dueDate {
            subtitle = "Add reminder · due \(Self.formatDue(due))"
        } else {
            subtitle = "Add reminder · no due date"
        }
        let id = ResultID(module: Self.manifest.identifier, key: "capture")
        let payload = (try? ModuleActionCoding.encode(TodoAction.create(title: original))) ?? Data()
        return ResultItem(
            id: id,
            title: titleText,
            titleAttributed: AttributedString(titleText),
            subtitle: subtitle,
            icon: .symbol("plus.circle.fill"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "create"),
                title: "Add Reminder",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func dueRow(_ reminder: ReminderSnapshot) -> ResultItem {
        let subtitle: String
        if let date = reminder.dueDate {
            subtitle = "Due \(Self.formatDue(date)) · \(reminder.calendarTitle)"
        } else {
            subtitle = reminder.calendarTitle
        }
        let id = ResultID(module: Self.manifest.identifier, key: "due.\(reminder.id)")
        let payload = (try? ModuleActionCoding.encode(TodoAction.complete(id: reminder.id))) ?? Data()
        return ResultItem(
            id: id,
            title: reminder.title,
            titleAttributed: AttributedString(reminder.title),
            subtitle: subtitle,
            icon: .symbol("checkmark.circle"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "complete.\(reminder.id)"),
                title: "Mark Complete",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func openDetailRow(dueCount: Int) -> ResultItem {
        let subtitle: String
        if dueCount == 0 {
            subtitle = "Today, Upcoming, Inbox, and Completed"
        } else if dueCount == 1 {
            subtitle = "1 due today · open full list"
        } else {
            subtitle = "\(dueCount) due today · open full list"
        }
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "open-detail"),
            title: "Open Todo",
            titleAttributed: AttributedString("Open Todo"),
            subtitle: subtitle,
            icon: .symbol("checklist"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open detail",
                kind: .openModuleDetail(.todo, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 2),
        )
    }

    private func emptyTodayRow() -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "today.empty")
        return ResultItem(
            id: id,
            title: "Nothing due today",
            titleAttributed: AttributedString("Nothing due today"),
            subtitle: "Type a task to add it",
            icon: .symbol("checkmark.seal"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "noop"),
                title: "OK",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .informational
        )
    }

    private func permissionRow(authorization: RemindersAuthorization) -> ResultItem {
        let requestPayload = (try? ModuleActionCoding.encode(TodoAction.requestAccess)) ?? Data()
        let settingsPayload = (try? ModuleActionCoding.encode(TodoAction.grant)) ?? Data()
        return PermissionResultBuilder.row(
            spec: PermissionCardSpec(
                module: Self.manifest.identifier,
                title: "Reminders access needed",
                explanation: "Luma can list today's tasks and add reminders from the launcher",
                icon: .symbol("checklist"),
                requestAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "request"),
                    title: "Allow Reminders Access",
                    kind: .custom(payload: requestPayload, handler: Self.manifest.identifier)
                ),
                settingsAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "grant"),
                    title: "Open System Settings",
                    kind: .custom(payload: settingsPayload, handler: Self.manifest.identifier)
                ),
                accessDenied: authorization == .denied
            )
        )
    }

    public static func captureStatusMessage(
        for parsed: TodoTimeParser.Parsed,
        now: Date = Date(),
        calendar: Calendar = .current
    ) -> String {
        guard let due = parsed.dueDate else { return "Added to Inbox" }
        return "Added for \(formatDue(due, now: now, calendar: calendar))"
    }

    public static func formatDue(_ date: Date, now: Date = Date(), calendar: Calendar = .current) -> String {
        let formatter = DateFormatter()
        formatter.calendar = calendar
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = calendar.timeZone
        if calendar.isDate(date, inSameDayAs: now) {
            formatter.dateFormat = "'today' HH:mm"
        } else if let tomorrow = calendar.date(byAdding: .day, value: 1, to: calendar.startOfDay(for: now)),
                  calendar.isDate(date, inSameDayAs: tomorrow) {
            formatter.dateFormat = "'tomorrow' HH:mm"
        } else {
            formatter.dateFormat = "MMM d HH:mm"
        }
        return formatter.string(from: date)
    }
}
