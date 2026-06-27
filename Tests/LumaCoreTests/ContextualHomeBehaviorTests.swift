import Foundation
import LumaCore
import Testing

@Test func contextualHomeTodoRowPrefersOpenOverComplete() {
    let item = ResultItem(
        id: ResultID(module: .todo, key: "contextual.open.test"),
        title: "Buy milk",
        titleAttributed: AttributedString("Buy milk"),
        subtitle: "Due today",
        icon: .symbol("checkmark.circle"),
        primaryAction: Action(
            id: ActionID(module: .todo, key: "contextual.open"),
            title: "Open Todo",
            kind: .openModuleDetail(.todo, payload: nil)
        ),
        secondaryActions: [
            Action(
                id: ActionID(module: .todo, key: "contextual.complete"),
                title: "Mark Complete",
                kind: .custom(payload: Data(), handler: .todo)
            )
        ],
        rankingHints: RankingHints(basePriority: 86),
        rowKind: .starter
    )
    #expect(item.primaryAction.title == "Open Todo")
    #expect(item.secondaryActions.first?.title == "Mark Complete")
    #expect(item.returnHint == "Open Todo")
}
