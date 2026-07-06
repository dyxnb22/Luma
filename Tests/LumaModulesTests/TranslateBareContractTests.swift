import Foundation
import LumaCore
import LumaModules
import Testing

@Test func translateBareReturnsOpenDetailStarter() async {
    let module = TranslateModule()
    let parsed = ParsedCommand(trigger: "tr", payload: "", module: .translate)
    let result = await module.handle(
        Query(raw: "tr", sequence: 1, command: parsed),
        context: QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(60)))
    )
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Open Translate")
    if case .openModuleDetail(let moduleID, _) = result.items.first?.primaryAction.kind {
        #expect(moduleID == .translate)
    } else {
        Issue.record("Expected openModuleDetail action")
    }
}
