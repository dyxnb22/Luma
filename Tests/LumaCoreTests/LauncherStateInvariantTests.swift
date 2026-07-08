import Foundation
import Testing
import LumaCore

@Test func invariantDetectsDetailModeWithoutContentDetail() {
    let snapshot = LauncherStateSnapshot(
        generatedAt: "2026-07-07T00:00:00Z",
        reason: "test",
        panel: .init(
            panelVisible: true,
            visibilitySessionVisible: true,
            visibilityGeneration: 1,
            isKeyWindow: true,
            firstResponderChain: "LumaSearchTextField"
        ),
        search: .init(
            visibleQuery: "",
            persistedQuery: "n",
            isDetailModeActive: true,
            isEditable: false,
            placeholder: "Notes"
        ),
        content: .init(
            modeKind: .home,
            detailModuleID: nil,
            showingDetail: false,
            showingResults: false,
            selectedIndex: 0,
            selectedItemID: nil,
            currentDetailModuleID: nil
        ),
        chrome: .init(
            detailContainerHidden: true,
            detailContainerAlpha: 0,
            splitColumnActive: true,
            splitRightPane: .guide,
            homeVisible: true,
            resultsVisible: false,
            hintContext: "detail"
        ),
        animation: .init(
            detailPresentationGeneration: 1,
            crossfadeGeneration: 1,
            detailCloseCrossfadeInFlight: false
        ),
        lastKeyboardCommand: nil,
        searchFieldCanBecomeFirstResponder: false
    )

    let violations = LauncherStateInvariantChecker.check(snapshot)
    #expect(violations.contains(.detailModeWithoutContentDetail))
}

@Test func invariantDetectsReopenDetailPlaceholderWithGuidePane() {
    let snapshot = LauncherStateSnapshot(
        generatedAt: "2026-07-07T00:00:00Z",
        reason: "test",
        panel: .init(
            panelVisible: true,
            visibilitySessionVisible: true,
            visibilityGeneration: 3,
            isKeyWindow: true,
            firstResponderChain: "LumaSearchTextField"
        ),
        search: .init(
            visibleQuery: "",
            persistedQuery: "n",
            isDetailModeActive: true,
            isEditable: false,
            placeholder: "Notes"
        ),
        content: .init(
            modeKind: .home,
            detailModuleID: nil,
            showingDetail: false,
            showingResults: false,
            selectedIndex: 0,
            selectedItemID: nil,
            currentDetailModuleID: nil
        ),
        chrome: .init(
            detailContainerHidden: true,
            detailContainerAlpha: 0,
            splitColumnActive: true,
            splitRightPane: .guide,
            homeVisible: true,
            resultsVisible: false,
            hintContext: "detail"
        ),
        animation: .init(
            detailPresentationGeneration: 0,
            crossfadeGeneration: 0,
            detailCloseCrossfadeInFlight: false
        ),
        lastKeyboardCommand: "cmdSpaceShow",
        searchFieldCanBecomeFirstResponder: false
    )

    let violations = LauncherStateInvariantChecker.check(snapshot)
    #expect(violations.contains(.reopenDetailPlaceholderWithGuidePane))
}

@Test func invariantAllowsHiddenPanelPreservedDetailMode() {
    let snapshot = LauncherStateSnapshot(
        generatedAt: "2026-07-07T00:00:00Z",
        reason: "test",
        panel: .init(
            panelVisible: false,
            visibilitySessionVisible: false,
            visibilityGeneration: 2,
            isKeyWindow: false,
            firstResponderChain: "nil"
        ),
        search: .init(
            visibleQuery: "",
            persistedQuery: "n",
            isDetailModeActive: true,
            isEditable: false,
            placeholder: "Notes"
        ),
        content: .init(
            modeKind: .detail,
            detailModuleID: "luma.notes",
            showingDetail: true,
            showingResults: false,
            selectedIndex: 0,
            selectedItemID: nil,
            currentDetailModuleID: "luma.notes"
        ),
        chrome: .init(
            detailContainerHidden: false,
            detailContainerAlpha: 1,
            splitColumnActive: true,
            splitRightPane: .detail,
            homeVisible: false,
            resultsVisible: false,
            hintContext: "detail"
        ),
        animation: .init(
            detailPresentationGeneration: 2,
            crossfadeGeneration: 2,
            detailCloseCrossfadeInFlight: false
        ),
        lastKeyboardCommand: "cmdSpaceHide",
        searchFieldCanBecomeFirstResponder: false
    )

    #expect(!LauncherStateInvariantChecker.check(snapshot).contains(.reopenDetailPlaceholderWithGuidePane))
}

@Test func invariantPassesCleanHomeState() {
    let snapshot = LauncherStateSnapshot(
        generatedAt: "2026-07-07T00:00:00Z",
        reason: "test",
        panel: .init(
            panelVisible: true,
            visibilitySessionVisible: true,
            visibilityGeneration: 1,
            isKeyWindow: true,
            firstResponderChain: "LumaSearchTextField"
        ),
        search: .init(
            visibleQuery: "",
            persistedQuery: "",
            isDetailModeActive: false,
            isEditable: true,
            placeholder: nil
        ),
        content: .init(
            modeKind: .home,
            detailModuleID: nil,
            showingDetail: false,
            showingResults: false,
            selectedIndex: 0,
            selectedItemID: nil,
            currentDetailModuleID: nil
        ),
        chrome: .init(
            detailContainerHidden: true,
            detailContainerAlpha: 0,
            splitColumnActive: true,
            splitRightPane: .guide,
            homeVisible: true,
            resultsVisible: false,
            hintContext: "home"
        ),
        animation: .init(
            detailPresentationGeneration: 0,
            crossfadeGeneration: 0,
            detailCloseCrossfadeInFlight: false
        ),
        lastKeyboardCommand: nil,
        searchFieldCanBecomeFirstResponder: true
    )

    #expect(LauncherStateInvariantChecker.check(snapshot).isEmpty)
}

@Test func escapePlannerUsesDetailContextWhenCoordinatorNotInDetail() {
    let step = LauncherEscapePlanner.nextStep(
        actionPanelVisible: false,
        showingDetail: false,
        detailContextActive: true,
        showingResults: false,
        queryTrimmedIsEmpty: true
    )
    #expect(step == .detailEscapeOrExit)
}

@Test func invariantDetectsDetailContentWithGuideSplitPane() {
    let snapshot = LauncherStateSnapshot(
        generatedAt: "2026-07-07T00:00:00Z",
        reason: "test",
        panel: .init(
            panelVisible: true,
            visibilitySessionVisible: true,
            visibilityGeneration: 1,
            isKeyWindow: true,
            firstResponderChain: "LumaSearchTextField"
        ),
        search: .init(
            visibleQuery: "",
            persistedQuery: "",
            isDetailModeActive: true,
            isEditable: false,
            placeholder: "Notes"
        ),
        content: .init(
            modeKind: .detail,
            detailModuleID: "notes",
            showingDetail: true,
            showingResults: false,
            selectedIndex: 0,
            selectedItemID: nil,
            currentDetailModuleID: "notes"
        ),
        chrome: .init(
            detailContainerHidden: false,
            detailContainerAlpha: 1,
            splitColumnActive: true,
            splitRightPane: .guide,
            homeVisible: false,
            resultsVisible: false,
            hintContext: "detail"
        ),
        animation: .init(
            detailPresentationGeneration: 1,
            crossfadeGeneration: 1,
            detailCloseCrossfadeInFlight: false
        ),
        lastKeyboardCommand: nil,
        searchFieldCanBecomeFirstResponder: false
    )

    #expect(LauncherStateInvariantChecker.check(snapshot).contains(.detailContentWithGuideSplitPane))
}
