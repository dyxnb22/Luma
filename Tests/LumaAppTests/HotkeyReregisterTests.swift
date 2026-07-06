import Foundation
import Testing
@testable import LumaApp

@Test @MainActor func hotkeyRegisterIsRegisteredTogglesCorrectly() throws {
    let controller = try HotkeyController(onPress: {})
    let combo = KeyCombo(virtualKeyCode: 49, carbonModifiers: 1 << 8)
    try controller.register(combo)
    #expect(controller.isRegistered)
    try controller.register(combo)
    #expect(controller.isRegistered)
    controller.unregister()
    #expect(!controller.isRegistered)
}
