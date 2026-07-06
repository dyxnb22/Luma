import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private actor MutableRemindersClient: RemindersClient {
    var authorizationState: RemindersAuthorization = .authorized
    private var continuations: [UUID: AsyncStream<Void>.Continuation] = [:]

    func authorization() async -> RemindersAuthorization {
        authorizationState
    }

    func requestAccess() async -> RemindersAuthorization {
        authorizationState
    }

    func todayDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        _ = now
        _ = limit
        return []
    }

    func noDueDate(limit: Int) async throws -> [ReminderSnapshot] {
        _ = limit
        return []
    }

    func futureDue(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        _ = now
        _ = limit
        return []
    }

    func completedRecently(now: Date, limit: Int) async throws -> [ReminderSnapshot] {
        _ = now
        _ = limit
        return []
    }

    func create(title: String, dueDate: Date?, notes: String?) async throws -> ReminderSnapshot {
        _ = title
        _ = dueDate
        _ = notes
        throw RemindersServiceError.accessDenied
    }

    func complete(id: String) async throws {
        _ = id
    }

    func uncomplete(id: String) async throws {
        _ = id
    }

    func update(id: String, title: String, dueDate: Date?, clearDueDate: Bool) async throws -> ReminderSnapshot {
        _ = id
        _ = title
        _ = dueDate
        _ = clearDueDate
        throw RemindersServiceError.accessDenied
    }

    func storeChanges() async -> AsyncStream<Void> {
        AsyncStream { continuation in
            let id = UUID()
            Task { await self.registerContinuation(id: id, continuation: continuation) }
            continuation.onTermination = { _ in
                Task { await self.removeContinuation(id: id) }
            }
        }
    }

    func publishStoreChange() async {
        for continuation in continuations.values {
            continuation.yield()
        }
    }

    func setAuthorization(_ state: RemindersAuthorization) async {
        authorizationState = state
    }

    private func registerContinuation(id: UUID, continuation: AsyncStream<Void>.Continuation) {
        continuations[id] = continuation
    }

    private func removeContinuation(id: UUID) {
        continuations.removeValue(forKey: id)
    }
}

@Test func todoModuleRefreshesCachedAuthorizationOnStoreChanges() async {
    let reminders = MutableRemindersClient()
    await reminders.setAuthorization(.authorized)
    let module = TodoModule()
    let context = ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        reminders: reminders
    )
    await module.warmup(context)

    let authorized = await module.handle(
        Query(raw: "todo", sequence: 1),
        context: QueryContext(deadline: .now + .seconds(1))
    )
    #expect(authorized.items.contains(where: { $0.title == "Reminders access needed" }) == false)

    await reminders.setAuthorization(.denied)
    await reminders.publishStoreChange()
    try? await Task.sleep(for: .milliseconds(50))

    let denied = await module.handle(
        Query(raw: "todo", sequence: 2),
        context: QueryContext(deadline: .now + .seconds(1))
    )
    #expect(denied.items.contains(where: { $0.title == "Reminders access needed" }))
}
