import Foundation
import Testing
import LumaCore

private struct StubHomeProvider: LauncherHomeProvider {
    let rows: [ResultItem]
    var warming: Bool = false

    func items() async -> [ResultItem] { rows }
    func isWarming() async -> Bool { warming }
}

@Test func homeAggregatorMergesProvidersAndFiltersEmptySections() async {
    let appsID = ModuleIdentifier(rawValue: "luma.apps")
    let openApps = StubHomeProvider(rows: [
        ResultItem(
            id: ResultID(module: appsID, key: "a"),
            title: "Safari",
            titleAttributed: "Safari",
            icon: .symbol("app"),
            primaryAction: Action(id: ActionID(module: appsID, key: "a"), title: "Open", kind: .noop),
            rankingHints: RankingHints()
        )
    ])

    let aggregator = LauncherHomeAggregator(openApps: openApps)
    let snapshot = await aggregator.snapshot()
    #expect(snapshot.sections.count == 1)
    #expect(snapshot.sections[0].kind == .openApps)
    #expect(snapshot.sections[0].items.count == 1)
}

@Test func homeAggregatorKeepsOpenAppsSectionWhenEmpty() async {
    let openApps = StubHomeProvider(rows: [])
    let aggregator = LauncherHomeAggregator(openApps: openApps)
    let snapshot = await aggregator.snapshot()
    #expect(snapshot.sections.count == 1)
    #expect(snapshot.sections[0].kind == .openApps)
    #expect(snapshot.sections[0].items.isEmpty)
    #expect(snapshot.sections[0].isWarming == false)
}

@Test func homeAggregatorMarksWarmingWhenProviderReportsWarming() async {
    let openApps = StubHomeProvider(rows: [], warming: true)
    let aggregator = LauncherHomeAggregator(openApps: openApps)
    let snapshot = await aggregator.snapshot()
    #expect(snapshot.sections[0].isWarming == true)
}

@Test func homeSnapshotFlatItemsPreservesSectionOrder() async {
    let appsID = ModuleIdentifier(rawValue: "luma.apps")
    let todoID = ModuleIdentifier(rawValue: "luma.todo")
    let itemA = ResultItem(
        id: ResultID(module: appsID, key: "a"),
        title: "A",
        titleAttributed: "A",
        icon: .none,
        primaryAction: Action(id: ActionID(module: appsID, key: "a"), title: "Open", kind: .noop),
        rankingHints: RankingHints()
    )
    let itemB = ResultItem(
        id: ResultID(module: todoID, key: "b"),
        title: "B",
        titleAttributed: "B",
        icon: .none,
        primaryAction: Action(id: ActionID(module: todoID, key: "b"), title: "Open", kind: .noop),
        rankingHints: RankingHints()
    )
    let snapshot = LauncherHomeSnapshot(sections: [
        LauncherHomeSection(kind: .openApps, items: [itemA, itemB])
    ])
    #expect(snapshot.flatItems.map(\.title) == ["A", "B"])
}
