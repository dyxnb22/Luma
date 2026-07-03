import Foundation
import Testing
import LumaCore

private struct StubHomeProvider: LauncherHomeProvider {
    let rows: [ResultItem]

    func items() async -> [ResultItem] { rows }
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

    let aggregator = LauncherHomeAggregator(
        openApps: openApps,
        setup: nil
    )
    let snapshot = await aggregator.snapshot()
    #expect(snapshot.sections.count == 1)
    #expect(snapshot.sections[0].kind == .openApps)
    #expect(snapshot.sections[0].items.count == 1)
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
        LauncherHomeSection(kind: .openApps, items: [itemA]),
        LauncherHomeSection(kind: .create, items: [itemB])
    ])
    #expect(snapshot.flatItems.map(\.title) == ["A", "B"])
}
