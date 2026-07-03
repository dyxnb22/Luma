import Foundation
import Testing
import LumaCore

@Test func escapePlannerDismissesActionPanelFirst() {
    let step = LauncherEscapePlanner.nextStep(
        actionPanelVisible: true,
        showingDetail: true,
        showingResults: true,
        queryTrimmedIsEmpty: false
    )
    #expect(step == .dismissActionPanel)
}

@Test func escapePlannerExitsDetailBeforeHome() {
    let step = LauncherEscapePlanner.nextStep(
        actionPanelVisible: false,
        showingDetail: true,
        showingResults: false,
        queryTrimmedIsEmpty: true
    )
    #expect(step == .detailEscapeOrExit)
}

@Test func escapePlannerShowsHomeWhenSearching() {
    let step = LauncherEscapePlanner.nextStep(
        actionPanelVisible: false,
        showingDetail: false,
        showingResults: true,
        queryTrimmedIsEmpty: true
    )
    #expect(step == .showHome)
}

@Test func escapePlannerShowsHomeForNonEmptyQuery() {
    let step = LauncherEscapePlanner.nextStep(
        actionPanelVisible: false,
        showingDetail: false,
        showingResults: false,
        queryTrimmedIsEmpty: false
    )
    #expect(step == .showHome)
}

@Test func escapePlannerDismissesPanelFromIdleHome() {
    let step = LauncherEscapePlanner.nextStep(
        actionPanelVisible: false,
        showingDetail: false,
        showingResults: false,
        queryTrimmedIsEmpty: true
    )
    #expect(step == .dismissPanel)
}
