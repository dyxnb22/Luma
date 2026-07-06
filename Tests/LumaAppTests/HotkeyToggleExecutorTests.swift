import Foundation
import Testing
@testable import LumaApp

@Test @MainActor func hotkeySchedulePressInvokesMainActorHandler() async throws {
    var called = false
    let controller = try HotkeyController { called = true }
    controller.schedulePress()
    try await Task.sleep(for: .milliseconds(20))
    #expect(called)
}
