import Foundation
import LumaCore
import Testing
@testable import LumaApp

@Test @MainActor func taskRegistryCancelsRegisteredWork() async {
    let registry = LauncherTaskRegistry()
    var finished = false
    let task = Task {
        try? await Task.sleep(for: .seconds(60))
        finished = true
    }
    registry.register(key: "test", task: task)
    registry.cancelAll()
    try? await Task.sleep(for: .milliseconds(20))
    #expect(finished == false)
    #expect(registry.contains(key: "test") == false)
}
