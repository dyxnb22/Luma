import Foundation
import Testing
import LumaCore

@Test func homeSplitShowsGuideOnEmptyQueryHome() {
    let state = LauncherHomeSplitPlanner.layout(
        queryTrimmedIsEmpty: true,
        showingDetail: false,
        showingResults: false
    )
    #expect(state.columnSplitActive)
    #expect(state.rightPane == .guide)
}

@Test func homeSplitShowsDetailWhileDetailOpen() {
    let state = LauncherHomeSplitPlanner.layout(
        queryTrimmedIsEmpty: true,
        showingDetail: true,
        showingResults: false
    )
    #expect(state.columnSplitActive)
    #expect(state.rightPane == .detail)
}

@Test func homeSplitCollapsesWhenTyping() {
    let state = LauncherHomeSplitPlanner.layout(
        queryTrimmedIsEmpty: false,
        showingDetail: false,
        showingResults: true
    )
    #expect(!state.columnSplitActive)
    #expect(state.rightPane == .hidden)
}

@Test func homeSplitCollapsesForNonemptyQueryEvenWithoutResults() {
    let state = LauncherHomeSplitPlanner.layout(
        queryTrimmedIsEmpty: false,
        showingDetail: true,
        showingResults: false
    )
    #expect(!state.columnSplitActive)
    #expect(state.rightPane == .hidden)
}
