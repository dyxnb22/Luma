import Foundation
import Testing
@testable import LumaCore

@Test func querySnapshotCacheStoresAndReturnsSnapshot() async {
    let cache = QuerySnapshotCache()
    let snapshot = ResultSnapshot(querySequence: 1, items: [])
    await cache.store(normalizedQuery: "safari", moduleGeneration: 7, snapshot: snapshot)
    let hit = await cache.lookup(normalizedQuery: "safari", moduleGeneration: 7)
    #expect(hit?.querySequence == 1)
    let miss = await cache.lookup(normalizedQuery: "safari", moduleGeneration: 8)
    #expect(miss == nil)
}

@Test func querySnapshotCacheRejectsSecretsModuleRows() async {
    let cache = QuerySnapshotCache()
    let item = ResultItem(
        id: ResultID(module: ModuleIdentifier(rawValue: "luma.secrets"), key: "x"),
        title: "vault",
        titleAttributed: AttributedString("vault"),
        icon: .none,
        primaryAction: Action(id: ActionID(module: .secrets, key: "x"), title: "x", kind: .noop),
        rankingHints: RankingHints()
    )
    let snapshot = ResultSnapshot(querySequence: 2, items: [item])
    await cache.store(normalizedQuery: "sec", moduleGeneration: 1, snapshot: snapshot)
    let hit = await cache.lookup(normalizedQuery: "sec", moduleGeneration: 1)
    #expect(hit == nil)
}

@Test func querySnapshotCacheStoresMixedGlobalSnapshot() async {
    let cache = QuerySnapshotCache()
    let appsRow = ResultItem(
        id: ResultID(module: .apps, key: "safari"),
        title: "Safari",
        titleAttributed: AttributedString("Safari"),
        icon: .none,
        primaryAction: Action(id: ActionID(module: .apps, key: "safari"), title: "Open", kind: .noop),
        rankingHints: RankingHints()
    )
    let clipboardRow = ResultItem(
        id: ResultID(module: .clipboard, key: "clip-1"),
        title: "copied text",
        titleAttributed: AttributedString("copied text"),
        icon: .none,
        primaryAction: Action(id: ActionID(module: .clipboard, key: "clip-1"), title: "Copy", kind: .noop),
        rankingHints: RankingHints()
    )
    let snapshot = ResultSnapshot(querySequence: 3, items: [appsRow, clipboardRow])
    await cache.store(normalizedQuery: "copy", moduleGeneration: 2, snapshot: snapshot)
    let hit = await cache.lookup(normalizedQuery: "copy", moduleGeneration: 2)
    #expect(hit?.items.count == 2)
    #expect(hit?.items.map(\.id.module) == [.apps, .clipboard])
}
