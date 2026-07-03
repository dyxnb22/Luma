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

@Test func workbenchCommandHintMatchesCaptureRoute() {
    let router = WorkbenchCommandRouter()
    let route = router.route(raw: "cap clip note")
    guard case .capture(let definition) = route else {
        Issue.record("Expected capture route")
        return
    }
    let hint = router.commandHint(for: route, raw: "cap clip note")
    #expect(hint?.usageFormat == definition.triggers.first)
    #expect(hint?.description == definition.title)
}

@Test func workbenchCommandHintMatchesProjectWorkRoute() {
    let router = WorkbenchCommandRouter()
    let route = router.route(raw: "proj work")
    let hint = router.commandHint(for: route, raw: "proj work")
    #expect(hint?.usageFormat == "proj work")
    #expect(hint?.description == WorkbenchEmptyStateCopy.openProjectWorkspace)
}
