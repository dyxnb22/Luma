import Foundation
import LumaCore
import Testing

@Test func sessionStateRejectsDetailOpenWhileActive() {
    var state = LauncherSessionState(
        content: .detail(.clipboard),
        detailMode: .active(suspendedQuery: "clip")
    )
    let effects = state.apply(.detailOpenRequested(.notes))
    #expect(effects.isEmpty)
    #expect(state.content == .detail(.clipboard))
}

@Test func sessionStateDetailCloseClearsDetailMode() {
    var state = LauncherSessionState(
        content: .detail(.clipboard),
        detailMode: .active(suspendedQuery: "clip")
    )
    _ = state.apply(.detailExitRequested(.returnToHome(crossfadeToGuide: true)))
    let effects = state.apply(.detailClosed)
    #expect(state.detailMode == .inactive)
    #expect(state.content == .home)
    #expect(effects.contains(.clearDetailModeState))
}

@Test func sessionStateUserTypingExitsDetail() {
    var state = LauncherSessionState(
        content: .detail(.clipboard),
        detailMode: .active(suspendedQuery: nil)
    )
    let effects = state.apply(.userTypedInDetail)
    #expect(state.content == .results)
    #expect(state.detailMode == .inactive)
    #expect(effects.contains(.clearDetailModeState))
}

@Test func sessionStatePanelHideCancelsTasks() {
    var state = LauncherSessionState(panel: .visible)
    let effects = state.apply(.panelHideBegan)
    #expect(state.panel == .hiding)
    #expect(effects.contains(.cancelAllTasks))
}

@Test func sessionStateRejectsNonemptyQueryWhileInDetail() {
    var state = LauncherSessionState(
        content: .detail(.notes),
        detailMode: .active(suspendedQuery: nil)
    )
    let effects = state.apply(.queryBecameNonempty)
    #expect(effects.isEmpty)
    #expect(state.content == .detail(.notes))
}
