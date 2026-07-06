import CoreGraphics
import Foundation
import LumaCore
import Testing

@Test func panelRepositionWhenVisibleFrameChanges() {
    let last = CGRect(x: 0, y: 0, width: 1920, height: 1080)
    let next = CGRect(x: 100, y: 0, width: 1600, height: 900)
    #expect(
        LauncherPanelRepositionPolicy.shouldReposition(
            isPanelVisible: true,
            lastVisibleFrame: last,
            newVisibleFrame: next
        )
    )
}

@Test func panelSkipsRepositionWhenHidden() {
    let frame = CGRect(x: 0, y: 0, width: 1920, height: 1080)
    #expect(
        !LauncherPanelRepositionPolicy.shouldReposition(
            isPanelVisible: false,
            lastVisibleFrame: frame,
            newVisibleFrame: frame
        )
    )
}

@Test func panelSkipsRepositionWhenFrameUnchanged() {
    let frame = CGRect(x: 0, y: 0, width: 1920, height: 1080)
    #expect(
        !LauncherPanelRepositionPolicy.shouldReposition(
            isPanelVisible: true,
            lastVisibleFrame: frame,
            newVisibleFrame: frame
        )
    )
}

@Test func panelRepositionsWhenFirstShownOnScreen() {
    let frame = CGRect(x: 0, y: 0, width: 1920, height: 1080)
    #expect(
        LauncherPanelRepositionPolicy.shouldReposition(
            isPanelVisible: true,
            lastVisibleFrame: nil,
            newVisibleFrame: frame
        )
    )
}
