import CoreGraphics
import Testing
@testable import LumaModules

@Test func windowLayoutComputesHalvesAndCenter() {
    let screen = CGRect(x: 0, y: 0, width: 1200, height: 800)
    #expect(WindowLayoutEngine.frame(for: .leftHalf, screen: screen) == CGRect(x: 0, y: 0, width: 600, height: 800))
    #expect(WindowLayoutEngine.frame(for: .rightHalf, screen: screen) == CGRect(x: 600, y: 0, width: 600, height: 800))
    #expect(WindowLayoutEngine.frame(for: .maximize, screen: screen) == screen)

    let center = WindowLayoutEngine.frame(for: .center, screen: screen)
    #expect(center.width == 864)
    #expect(center.height == 576)
}

@Test func windowLayoutHandlesOffsetScreen() {
    let screen = CGRect(x: 100, y: 50, width: 1000, height: 700)
    #expect(WindowLayoutEngine.frame(for: .bottomHalf, screen: screen).origin == CGPoint(x: 100, y: 50))
    #expect(WindowLayoutEngine.frame(for: .topHalf, screen: screen).origin == CGPoint(x: 100, y: 400))
}

@Test func windowLayoutHandlesSmallScreen() {
    let screen = CGRect(x: 0, y: 0, width: 320, height: 240)
    let center = WindowLayoutEngine.frame(for: .center, screen: screen)
    #expect(center.width == 230.39999999999998)
    #expect(center.height == 172.79999999999998)
}
