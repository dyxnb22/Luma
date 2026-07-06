import Foundation
import LumaCore
import Testing

private func sampleItem(key: String) -> ResultItem {
    ResultItem(
        id: ResultID(module: .apps, key: key),
        title: key,
        titleAttributed: AttributedString(key),
        icon: .none,
        primaryAction: Action(
            id: ActionID(module: .apps, key: key),
            title: key,
            kind: .noop
        ),
        rankingHints: RankingHints(basePriority: 0)
    )
}

@Test func canReorderRowsRejectsDuplicateIdentityKeys() {
    let item = sampleItem(key: "dup")
    let rows: [LauncherListRows.Row] = [
        .init(kind: .item(item, flatIndex: 0)),
        .init(kind: .item(item, flatIndex: 1))
    ]
    #expect(LauncherListRowReuse.canReorderRows(rows, rows) == false)
}

@Test func canReorderRowsAcceptsPermutation() {
    let a = sampleItem(key: "a")
    let b = sampleItem(key: "b")
    let oldRows: [LauncherListRows.Row] = [
        .init(kind: .item(a, flatIndex: 0)),
        .init(kind: .item(b, flatIndex: 1))
    ]
    let newRows: [LauncherListRows.Row] = [
        .init(kind: .item(b, flatIndex: 0)),
        .init(kind: .item(a, flatIndex: 1))
    ]
    #expect(LauncherListRowReuse.canReorderRows(oldRows, newRows))
}
