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

// MARK: - Illegal transitions (I1–I7)

@Test func sessionStateI1RejectsPanelShowBeganWhenVisible() {
    var state = LauncherSessionState(panel: .visible)
    let effects = state.apply(.panelShowBegan)
    #expect(effects.isEmpty)
    #expect(state.panel == .visible)
}

@Test func sessionStateI2RejectsPanelShowCompletedWhenHidden() {
    var state = LauncherSessionState(panel: .hidden)
    let effects = state.apply(.panelShowCompleted)
    #expect(effects.isEmpty)
    #expect(state.panel == .hidden)
}

@Test func sessionStateI3RejectsPanelHideBeganWhenHidden() {
    var state = LauncherSessionState(panel: .hidden)
    let effects = state.apply(.panelHideBegan)
    #expect(effects.isEmpty)
    #expect(state.panel == .hidden)
}

@Test func sessionStateI4RejectsQueryBecameEmptyWhenHome() {
    var state = LauncherSessionState(content: .home)
    let effects = state.apply(.queryBecameEmpty)
    #expect(effects.isEmpty)
    #expect(state.content == .home)
}

@Test func sessionStateI5RejectsDetailExitWhenInactive() {
    var state = LauncherSessionState()
    let effects = state.apply(.detailExitRequested(.returnToHome(crossfadeToGuide: false)))
    #expect(effects.isEmpty)
    #expect(state.detailMode == .inactive)
}

@Test func sessionStateI6RejectsUserTypedInDetailWhenInactive() {
    var state = LauncherSessionState(content: .home)
    let effects = state.apply(.userTypedInDetail)
    #expect(effects.isEmpty)
    #expect(state.content == .home)
}

@Test func sessionStateI7RejectsPanelShowCompletedWhenVisible() {
    var state = LauncherSessionState(panel: .visible)
    let effects = state.apply(.panelShowCompleted)
    #expect(effects.isEmpty)
    #expect(state.panel == .visible)
}
