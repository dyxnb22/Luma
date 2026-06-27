import Foundation
import LumaCore
import LumaModules
import Testing

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
    let result = await module.handle(
        Query(raw: "doctor", sequence: 1),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(20)))
    )
    #expect(result.items.contains { $0.id.key == "doctor.commands" })
    #expect(result.items.contains { $0.id.key == "doctor.summary" })
}
