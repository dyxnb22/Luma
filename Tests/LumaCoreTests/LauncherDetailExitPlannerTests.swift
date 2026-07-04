import Foundation
import Testing
import LumaCore

@Test func detailExitRestoresSuspendedQuery() {
    let outcome = LauncherDetailExitPlanner.outcome(
        showingDetail: true,
        suspendedQuery: "clip jwt",
        columnSplitActive: true
    )
    #expect(outcome == .restoreSuspendedQuery("clip jwt"))
}

@Test func detailExitReturnsHomeWhenSuspendedQueryEmpty() {
    let outcome = LauncherDetailExitPlanner.outcome(
        showingDetail: true,
        suspendedQuery: "   ",
        columnSplitActive: true
    )
    #expect(outcome == .returnToHome(crossfadeToGuide: true))
}

@Test func detailExitCrossfadesOnlyOnColumnSplitHome() {
    let outcome = LauncherDetailExitPlanner.outcome(
        showingDetail: true,
        suspendedQuery: nil,
        columnSplitActive: false
    )
    #expect(outcome == .returnToHome(crossfadeToGuide: false))
}

@Test func detailExitReenablesSearchWhenDetailNotShowing() {
    let outcome = LauncherDetailExitPlanner.outcome(
        showingDetail: false,
        suspendedQuery: "clip",
        columnSplitActive: true
    )
    #expect(outcome == .reenableSearchOnly)
}
