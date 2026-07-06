import Foundation
import Testing
import LumaCore

@Test func detailTypingCancelClearsSuspendedQuery() {
    var state = LauncherSearchDetailModeState(visibleQuery: "clip foo", isEditable: true)
    state = LauncherSearchDetailMode.beginDetailMode(state, moduleTitle: "Clipboard")
    #expect(state.suspendedQuery == "clip foo")
    #expect(state.visibleQuery.isEmpty)

    state = LauncherSearchDetailMode.cancelDetailMode(state)
    #expect(state.suspendedQuery == nil)
    #expect(state.isEditable)
}

@Test func detailChromeExitRestoresSuspendedQuery() {
    var state = LauncherSearchDetailModeState(visibleQuery: "clip foo", isEditable: true)
    state = LauncherSearchDetailMode.beginDetailMode(state, moduleTitle: "Clipboard")

    let outcome = LauncherDetailExitPlanner.outcome(
        showingDetail: true,
        suspendedQuery: state.suspendedQuery,
        columnSplitActive: true
    )
    #expect(outcome == .restoreSuspendedQuery("clip foo"))

    let (next, restored) = LauncherSearchDetailMode.endDetailMode(state)
    #expect(restored == "clip foo")
    #expect(next.suspendedQuery == nil)
}

@Test func detailTypingThenEscapeDoesNotRestoreSuspendedQuery() {
    var state = LauncherSearchDetailModeState(visibleQuery: "clip foo", isEditable: true)
    state = LauncherSearchDetailMode.beginDetailMode(state, moduleTitle: "Clipboard")
    state = LauncherSearchDetailMode.cancelDetailMode(state)
    state.visibleQuery = "t buy milk"

    let escapeStep = LauncherEscapePlanner.nextStep(
        actionPanelVisible: false,
        showingDetail: false,
        showingResults: true,
        queryTrimmedIsEmpty: false
    )
    #expect(escapeStep == .showHome)

    let detailExit = LauncherDetailExitPlanner.outcome(
        showingDetail: false,
        suspendedQuery: state.suspendedQuery,
        columnSplitActive: false
    )
    #expect(detailExit == .reenableSearchOnly)
}

@Test func detailChromeExitReturnsHomeWhenSuspendedQueryEmpty() {
    var state = LauncherSearchDetailModeState(visibleQuery: "", isEditable: true)
    state = LauncherSearchDetailMode.beginDetailMode(state, moduleTitle: "Todo")
    let outcome = LauncherDetailExitPlanner.outcome(
        showingDetail: true,
        suspendedQuery: state.suspendedQuery,
        columnSplitActive: true
    )
    #expect(outcome == .returnToHome(crossfadeToGuide: true))
}
