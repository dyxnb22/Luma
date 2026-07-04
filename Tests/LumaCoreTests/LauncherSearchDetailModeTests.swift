import Foundation
import Testing
import LumaCore

@Test func searchDetailModeSuspendsAndClearsVisibleQuery() {
    let initial = LauncherSearchDetailModeState(visibleQuery: "note daily", isEditable: true)
    let next = LauncherSearchDetailMode.beginDetailMode(initial, moduleTitle: "Notes")
    #expect(next.suspendedQuery == "note daily")
    #expect(next.visibleQuery.isEmpty)
    #expect(!next.isEditable)
}

@Test func searchDetailModeEndRestoresSuspendedQueryAndEditable() {
    var state = LauncherSearchDetailModeState(visibleQuery: "", suspendedQuery: "tr hello", isEditable: false)
    let ended = LauncherSearchDetailMode.endDetailMode(state)
    state = ended.state
    #expect(ended.restoredQuery == "tr hello")
    #expect(state.isEditable)
    #expect(state.suspendedQuery == nil)
}

@Test func searchDetailModeCancelDropsSuspendedQuery() {
    let state = LauncherSearchDetailModeState(visibleQuery: "", suspendedQuery: "word", isEditable: false)
    let next = LauncherSearchDetailMode.cancelDetailMode(state)
    #expect(next.suspendedQuery == nil)
    #expect(next.isEditable)
}

@Test func searchDetailModeReenableOnlyWhenStuck() {
    let stuck = LauncherSearchDetailModeState(isEditable: false)
    let fixed = LauncherSearchDetailMode.reEnableSearchFieldIfNeeded(stuck)
    #expect(fixed.isEditable)

    let editable = LauncherSearchDetailModeState(isEditable: true)
    #expect(LauncherSearchDetailMode.reEnableSearchFieldIfNeeded(editable) == editable)
}
