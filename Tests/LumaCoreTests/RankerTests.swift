import Foundation
import Testing
@testable import LumaCore

@Test func rankerDropsNonMatchingQuery() {
    let module = ModuleIdentifier(rawValue: "test")
    let id = ResultID(module: module, key: "safari")
    let action = Action(id: ActionID(module: module, key: "noop"), title: "Noop", kind: .noop)
    let item = ResultItem(
        id: id,
        title: "Safari",
        titleAttributed: AttributedString("Safari"),
        icon: .none,
        primaryAction: action,
        rankingHints: RankingHints()
    )
    let query = Query(raw: "zzz", sequence: 1)
    #expect(Ranker.score(item: item, query: query, usage: nil) == -.infinity)
}
