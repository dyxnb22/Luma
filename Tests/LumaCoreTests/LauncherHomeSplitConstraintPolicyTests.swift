import Foundation
import Testing
import LumaCore

@Test func splitConstraintPolicyGuideHome() {
    let state = LauncherHomeSplitState(columnSplitActive: true, rightPane: .guide)
    let flags = LauncherHomeSplitConstraintPolicy.flags(for: state)
    #expect(flags.listSplitWidthActive)
    #expect(!flags.listFullWidthTrailingActive)
    #expect(!flags.detailRightColumnActive)
    #expect(flags.guideVisible)
    #expect(!flags.detailVisible)
    #expect(flags.dividerVisible)
}

@Test func splitConstraintPolicyDetailInRightColumn() {
    let state = LauncherHomeSplitState(columnSplitActive: true, rightPane: .detail)
    let flags = LauncherHomeSplitConstraintPolicy.flags(for: state)
    #expect(flags.listSplitWidthActive)
    #expect(!flags.listFullWidthTrailingActive)
    #expect(flags.detailRightColumnActive)
    #expect(!flags.guideVisible)
    #expect(flags.detailVisible)
    #expect(flags.dividerVisible)
}

@Test func splitConstraintPolicySingleColumnResults() {
    let state = LauncherHomeSplitState(columnSplitActive: false, rightPane: .hidden)
    let flags = LauncherHomeSplitConstraintPolicy.flags(for: state)
    #expect(!flags.listSplitWidthActive)
    #expect(flags.listFullWidthTrailingActive)
    #expect(!flags.detailRightColumnActive)
    #expect(!flags.guideVisible)
    #expect(!flags.detailVisible)
    #expect(!flags.dividerVisible)
}

@Test func splitPlannerAndConstraintPolicyStayAligned() {
    let plannerState = LauncherHomeSplitPlanner.layout(
        queryTrimmedIsEmpty: true,
        showingDetail: true,
        showingResults: false
    )
    let flags = LauncherHomeSplitConstraintPolicy.flags(for: plannerState)
    #expect(plannerState.columnSplitActive)
    #expect(plannerState.rightPane == .detail)
    #expect(flags.detailRightColumnActive)
    #expect(flags.detailVisible)
}
