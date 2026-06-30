import Foundation
import Testing
import LumaCore

@Test func workbenchCommandRouterMatchesCaptureCommands() {
    let router = WorkbenchCommandRouter()
    #expect(router.route(raw: "cap clip note") != .none)
    #expect(router.route(raw: "save url") != .none)
    #expect(router.route(raw: "continue project") == .continueProject)
    #expect(router.route(raw: "proj work") == .projectWork)
    #expect(router.route(raw: "attach project") == .attachProject)
    #expect(router.route(raw: "safari") == .none)
}

@Test func workbenchCommandRouteDoesNotMatchModuleQueries() {
    let router = WorkbenchCommandRouter()
    #expect(router.route(raw: "n daily") == .none)
}

@Test func workbenchCommandDefinitionRequiresTodoForClipTodo() {
    let definition = WorkbenchCommandRouter.defaultDefinitions.first { $0.id == .captureClipboardTodo }
    #expect(definition?.requiredModule == .workbenchTodo)
}
