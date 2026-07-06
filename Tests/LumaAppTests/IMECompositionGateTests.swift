import Foundation
import LumaCore
import Testing

@Test func queryDispatchBlockedWhileIMEComposing() {
    #expect(LauncherQueryDispatchPolicy.shouldDispatchQuery(isComposing: true) == false)
}

@Test func queryDispatchAllowedWhenIMECompositionCommitted() {
    #expect(LauncherQueryDispatchPolicy.shouldDispatchQuery(isComposing: false) == true)
}
