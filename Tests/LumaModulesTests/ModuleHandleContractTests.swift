import Foundation
import LumaCore
import LumaInfrastructure
import LumaServices
import Testing
@testable import LumaModules

@Test func commandsGlobalSearchDoesNotRunDoctorChecks() async {
    let ax = CountingAccessibilityClient()
    let module = CommandsModule()
    let query = Query(raw: "doctor", sequence: 1, command: nil)
    let context = QueryContext(
        deadline: .now + .seconds(1),
        platform: QueryPlatformClients(accessibility: ax)
    )
    let result = await module.handle(query, context: context)
    #expect(result.items.contains { $0.id.key == "doctor" } == false)
    #expect(ax.trustedCallCount == 0)
}

@Test func commandsExplicitDoctorPayloadRunsDoctorChecks() async throws {
    let ax = CountingAccessibilityClient()
    let module = CommandsModule()
    await module.warmup(ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: NoopPasteboardClient(),
        accessibility: ax,
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore()
    ))
    let payload = try ModuleActionCoding.encode(CommandsAction.doctor)
    let action = Action(
        id: ActionID(module: .commands, key: "doctor"),
        title: "Global Doctor",
        kind: .custom(payload: payload, handler: .commands)
    )
    try await module.perform(action, context: ActionContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        pasteboard: NoopPasteboardClient(),
        accessibility: ax
    ))
    #expect(ax.trustedCallCount >= 1)
}

@Test func snippetsHandleDoesNotAwaitAccessibility() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaModules/Snippets/SnippetsModule.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    let handleStart = source.range(of: "public func handle(")!
    let handleEnd = source[handleStart.lowerBound...].range(of: "public func perform(")!.lowerBound
    let handleSection = String(source[handleStart.lowerBound..<handleEnd])
    #expect(!handleSection.contains("await accessibility"))
    #expect(!handleSection.contains("accessibility.isTrusted"))
}

@Test func browserTabsHandleUsesCacheOnlyPath() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaModules/BrowserTabs/BrowserTabsModule.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    let handleStart = source.range(of: "public func handle(")!
    let handleEnd = source[handleStart.lowerBound...].range(of: "public func perform(")!.lowerBound
    let handleSection = String(source[handleStart.lowerBound..<handleEnd])
    #expect(handleSection.contains("await service.cachedTabs()"))
    #expect(!handleSection.contains("searchableTabs()"))
}

@Test func windowsModuleDeferredFromActiveBuiltIns() {
    let active = Set(BuiltInModules.makeAll().map { type(of: $0).manifest.identifier })
    #expect(!active.contains(.windows))
    let deferred = BuiltInModules.makeDeferred().map { type(of: $0).manifest.identifier }
    #expect(deferred.contains(.windows))
}

private final class CountingAccessibilityClient: AccessibilityClient, @unchecked Sendable {
    nonisolated(unsafe) var trustedCallCount = 0

    func isTrusted() async -> Bool {
        trustedCallCount += 1
        return false
    }

    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { false }
    func applyWindowLayout(_ preset: String) async {}
}
