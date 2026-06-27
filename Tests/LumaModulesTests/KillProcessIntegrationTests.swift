import Foundation
import Testing
import LumaCore
@testable import LumaModules
import LumaServices

@Test func killProcessFindsPreviewWhenRunning() async {
    let service = RunningProcessService()
    let records = await service.runningGUIApplications()
    let preview = records.filter { $0.bundleID == "com.apple.Preview" }
    #expect(!preview.isEmpty, "Preview must be running for this integration test")

    let module = KillProcessModule(service: service)
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(200)))
    let query = Query(
        raw: "kill preview",
        sequence: 1,
        command: ParsedCommand(trigger: "kill", payload: "preview", module: .killProcess)
    )
    let result = await module.handle(query, context: context)
    #expect(result.items.contains { $0.title == "预览" || $0.title.localizedCaseInsensitiveContains("Preview") })
}
