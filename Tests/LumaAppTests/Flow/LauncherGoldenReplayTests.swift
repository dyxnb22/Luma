import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

@Test @MainActor func launcherGoldenReplayEmptyQueryDoesNotSnapshot() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.activatePanel()
    harness.type("")
    try? await Task.sleep(for: .milliseconds(50))
    #expect(harness.lastSnapshot == nil)
}

@Test @MainActor func launcherGoldenReplayTargetedQueryProducesSnapshot() async {
    let harness = await LauncherFlowHarness.makeWithBuiltInModules()
    harness.activatePanel()
    harness.type("clip")
    try? await Task.sleep(for: .milliseconds(300))
    #expect(harness.lastSnapshot != nil)
}
