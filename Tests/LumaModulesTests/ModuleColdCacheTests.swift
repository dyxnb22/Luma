import Foundation
import LumaCore
import LumaServices
import Testing
@testable import LumaModules

@Test func menuItemsColdHandleSurfacesPermissionRequiredWhenAXDenied() async {
    let service = MenuBarTreeService()
    let module = MenuItemsModule(service: service)
    let parsed = ParsedCommand(trigger: "mb", payload: "fold", module: .menuItems)
    let query = Query(raw: "mb fold", sequence: 1, command: parsed)
    let context = QueryContext(
        deadline: .now + .seconds(1),
        platform: QueryPlatformClients(accessibility: UntrustedAccessibilityClient())
    )

    let result = await module.handle(query, context: context)

    #expect(result.items.isEmpty)
    #expect(result.diagnostic?.kind == .permissionRequired)
    #expect(result.diagnostic?.message.contains("Accessibility") == true)
}

@Test func menuItemsEmptyPayloadKeepsWarmingDiagnosticOnColdCache() async {
    let service = MenuBarTreeService()
    let module = MenuItemsModule(service: service)
    let parsed = ParsedCommand(trigger: "mb", payload: "", module: .menuItems)
    let query = Query(raw: "mb", sequence: 1, command: parsed)
    let context = QueryContext(
        deadline: .now + .seconds(1),
        platform: QueryPlatformClients(accessibility: UntrustedAccessibilityClient())
    )

    let result = await module.handle(query, context: context)

    #expect(result.items.isEmpty)
    #expect(result.diagnostic?.kind == .degraded)
    #expect(result.diagnostic?.message.contains("No cached menu items") == true)
}

@Test func todoColdHandleReturnsWarmingRowWithoutBlocking() async {
    let module = TodoModule()
    await module.configureRemindersForTesting(AuthorizedEmptyRemindersClient())

    let parsed = ParsedCommand(trigger: "t", payload: "", module: .todo)
    let query = Query(raw: "t", sequence: 1, command: parsed)
    let start = ContinuousClock.now
    let result = await module.handle(
        query,
        context: QueryContext(deadline: .now + .seconds(1))
    )
    let elapsedMs = elapsedMilliseconds(since: start)
    #expect(elapsedMs < 50)
    #expect(result.items.contains { $0.id.key == "warming" })
}

@Test func todoTeardownCancelsScheduledDueCacheRefresh() async {
    let module = TodoModule()
    await module.configureRemindersForTesting(SlowRemindersClient(delay: .milliseconds(250)))

    let parsed = ParsedCommand(trigger: "t", payload: "", module: .todo)
    let query = Query(raw: "t", sequence: 1, command: parsed)
    _ = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1)))
    await module.teardown()
    try? await Task.sleep(for: .milliseconds(300))

    #expect(await module.isDueCachePopulatedForTesting() == false)
}

@Test func wordbookColdHandleReturnsWarmingRowWithoutBlocking() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let module = WordbookModule(store: store)
    let parsed = ParsedCommand(trigger: "word", payload: "", module: .wordbook)
    let query = Query(raw: "word", sequence: 1, command: parsed)
    let start = ContinuousClock.now
    let result = await module.handle(
        query,
        context: QueryContext(deadline: .now + .seconds(1))
    )
    let elapsedMs = elapsedMilliseconds(since: start)
    #expect(elapsedMs < 50)
    #expect(result.items.first?.id.key == "warming")
}

@Test func wordbookTeardownCancelsScheduledDueCacheRefresh() async throws {
    let (store, url) = try WordbookTestFixtures.makeStore()
    defer { try? FileManager.default.removeItem(at: url) }

    let module = WordbookModule(store: store)
    let parsed = ParsedCommand(trigger: "word", payload: "", module: .wordbook)
    let query = Query(raw: "word", sequence: 1, command: parsed)
    _ = await module.handle(query, context: QueryContext(deadline: .now + .seconds(1)))
    await module.teardown()

    #expect(await module.isDueCacheRefreshInFlightForTesting() == false)
}

private func elapsedMilliseconds(since start: ContinuousClock.Instant) -> Double {
    let elapsed = start.duration(to: .now)
    return Double(elapsed.components.seconds) * 1000
        + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
}

private struct UntrustedAccessibilityClient: AccessibilityClient {
    func isTrusted() async -> Bool { false }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { false }
    func applyWindowLayout(_ preset: String) async {}
}

private struct NoopDatabaseClient: DatabaseClient {}

private struct AuthorizedEmptyRemindersClient: RemindersClient {
    func authorization() async -> RemindersAuthorization { .authorized }
    func requestAccess() async -> RemindersAuthorization { .authorized }
    func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func noDueDate(limit: Int) async throws -> [ReminderSnapshot] { [] }
    func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot {
        ReminderSnapshot(id: "1", title: title, dueDate: dueDate, isCompleted: false, calendarTitle: "Inbox")
    }
    func complete(id: String) async throws {}
    func uncomplete(id: String) async throws {}
    func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot {
        ReminderSnapshot(id: id, title: title, dueDate: dueDate, isCompleted: false, calendarTitle: "Inbox")
    }
    func storeChanges() async -> AsyncStream<Void> {
        AsyncStream { $0.finish() }
    }
}

private struct SlowRemindersClient: RemindersClient {
    let delay: Duration

    func authorization() async -> RemindersAuthorization { .authorized }
    func requestAccess() async -> RemindersAuthorization { .authorized }
    func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        try await Task.sleep(for: delay)
        return [ReminderSnapshot(id: "slow-1", title: "Slow", dueDate: nil, isCompleted: false, calendarTitle: "Inbox")]
    }
    func noDueDate(limit: Int) async throws -> [ReminderSnapshot] { [] }
    func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot] { [] }
    func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot {
        ReminderSnapshot(id: "1", title: title, dueDate: dueDate, isCompleted: false, calendarTitle: "Inbox")
    }
    func complete(id: String) async throws {}
    func uncomplete(id: String) async throws {}
    func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot {
        ReminderSnapshot(id: id, title: title, dueDate: dueDate, isCompleted: false, calendarTitle: "Inbox")
    }
    func storeChanges() async -> AsyncStream<Void> {
        AsyncStream { $0.finish() }
    }
}
