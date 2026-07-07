import Foundation
import LumaCore
import Testing

// Behavioral policy tests for C-UI-004. Does not exercise LauncherFlowHarness or
// LauncherRootController snapshot apply (C-TEST-004 production wiring divergence).

private func item(key: String) -> ResultItem {
    ResultItem(
        id: ResultID(module: .apps, key: key),
        title: key,
        titleAttributed: AttributedString(key),
        icon: .symbol("app"),
        primaryAction: Action(
            id: ActionID(module: .apps, key: key),
            title: key,
            kind: .noop
        ),
        rankingHints: RankingHints(basePriority: 0)
    )
}

@Test func selectionClampsWhenPriorIDRemovedFromSnapshot() {
    let selectable = [item(key: "a"), item(key: "b")]
    let index = LauncherListSelectionPreservePolicy.nextFlatIndex(
        preserveSelection: true,
        previousFlatIndex: 2,
        previousItemID: ResultID(module: .apps, key: "removed"),
        selectable: selectable
    )
    #expect(index == 1)
    #expect(index != 0)
}

@Test func selectionRestoresByIDWhenStillPresent() {
    let id = ResultID(module: .apps, key: "b")
    let selectable = [item(key: "a"), item(key: "b")]
    let index = LauncherListSelectionPreservePolicy.nextFlatIndex(
        preserveSelection: true,
        previousFlatIndex: 0,
        previousItemID: id,
        selectable: selectable
    )
    #expect(index == 1)
}

@Test func selectionWithoutPreserveResetsToZero() {
    let selectable = [item(key: "a"), item(key: "b")]
    let index = LauncherListSelectionPreservePolicy.nextFlatIndex(
        preserveSelection: false,
        previousFlatIndex: 2,
        previousItemID: ResultID(module: .apps, key: "removed"),
        selectable: selectable
    )
    #expect(index == 0)
}

@Test func returnActivationUsesClampedSelectionNotStaleIndex() {
    #expect(LauncherReturnActivationPolicy.outcome(itemCount: 2, selectedIndex: 1) == .activateSelected)
    #expect(LauncherReturnActivationPolicy.outcome(itemCount: 2, selectedIndex: 5) == .showNoResultsYet)
}

@Test func returnActivationShowsEmptyMessageWhenNoItems() {
    #expect(LauncherReturnActivationPolicy.outcome(itemCount: 0, selectedIndex: 0) == .showEmptyQueryMessage)
}

@Test func clampedIndexNeverJumpsToZeroWhenPriorRowShrinks() {
    let clamped = LauncherListSelectionPreservePolicy.clampedFlatIndex(2, selectableCount: 2)
    #expect(clamped == 1)
}
