import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

private final class CountingAccessibilityClient: AccessibilityClient, @unchecked Sendable {
    nonisolated(unsafe) var trustedCallCount = 0
    func isTrusted() async -> Bool {
        trustedCallCount += 1
        return true
    }
    func requestPermission() async {}
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
    func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { false }
    func applyWindowLayout(_ preset: String) async {}
}

@Test func projectActionManageActionsDoNotHideLauncher() {
    #expect(!ProjectAction.togglePin(path: "/a").hidesLauncher)
    #expect(!ProjectAction.updateOpener(path: "/a", opener: .cursor).hidesLauncher)
    #expect(!ProjectAction.addRoot("/dev").hidesLauncher)
    #expect(ProjectAction.open(path: "/a", opener: .cursor).hidesLauncher)
}

@Test func commandsModuleListDoesNotIncludeDoctorRunnable() async {
    let module = CommandsModule()
    let result = await module.handle(
        Query(raw: "cmd", sequence: 1, command: ParsedCommand(trigger: "cmd", payload: "", module: .commands)),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(20)))
    )
    #expect(!result.items.contains { $0.title == "Global Doctor" })
}

@Test func commandsModuleDoctorQueryReturnsDiagnostics() async {
    let module = CommandsModule()
    let parsed = ParsedCommand(trigger: "cmd", payload: "doctor", module: .commands)
    let handleResult = await module.handle(
        Query(raw: "cmd doctor", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(20)))
    )
    #expect(handleResult.items.contains { $0.id.key == "doctor" })

    let payload = (try? ModuleActionCoding.encode(CommandsAction.doctor)) ?? Data()
    let action = Action(
        id: ActionID(module: .commands, key: "doctor"),
        title: "Global Doctor",
        kind: .custom(payload: payload, handler: .commands)
    )
    let ax = CountingAccessibilityClient()
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
    try? await module.perform(action, context: ActionContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        pasteboard: NoopPasteboardClient(),
        accessibility: ax
    ))
    #expect(ax.trustedCallCount >= 1)
}
