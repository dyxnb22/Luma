import Foundation
import Testing
import LumaCore
import LumaModules

@Suite struct LauncherListRowReuseTests {
    private func itemRow(id: String, flatIndex: Int) -> LauncherListRows.Row {
        LauncherListRows.Row(kind: .item(
            ResultItem(
                id: ResultID(module: .apps, key: id),
                title: id,
                titleAttributed: AttributedString(id),
                subtitle: nil,
                icon: .symbol("app"),
                primaryAction: Action(id: ActionID(module: .apps, key: id), title: id, kind: .noop),
                rankingHints: RankingHints(basePriority: 0)
            ),
            flatIndex: flatIndex
        ))
    }

    @Test func canReuseWhenIdentityAndOrderMatch() {
        let oldRows = [
            LauncherListRows.Row(kind: .sectionHeader(title: "Open Apps", shortcutIndex: 1)),
            itemRow(id: "a", flatIndex: 0),
            itemRow(id: "b", flatIndex: 1)
        ]
        let newRows = [
            LauncherListRows.Row(kind: .sectionHeader(title: "Open Apps", shortcutIndex: 1)),
            itemRow(id: "a", flatIndex: 0),
            itemRow(id: "b", flatIndex: 1)
        ]
        #expect(LauncherListRowReuse.canReuseRows(oldRows, newRows))
    }

    @Test func cannotReuseWhenRowCountChanges() {
        let oldRows = [itemRow(id: "a", flatIndex: 0)]
        let newRows = [itemRow(id: "a", flatIndex: 0), itemRow(id: "b", flatIndex: 1)]
        #expect(!LauncherListRowReuse.canReuseRows(oldRows, newRows))
    }

    @Test func cannotReuseWhenItemIdentityChanges() {
        let oldRows = [itemRow(id: "a", flatIndex: 0)]
        let newRows = [itemRow(id: "b", flatIndex: 0)]
        #expect(!LauncherListRowReuse.canReuseRows(oldRows, newRows))
    }

    @Test func canReuseWhenOnlyFlatIndexChanges() {
        let oldRows = [itemRow(id: "a", flatIndex: 0), itemRow(id: "b", flatIndex: 1)]
        let newRows = [itemRow(id: "a", flatIndex: 1), itemRow(id: "b", flatIndex: 0)]
        #expect(LauncherListRowReuse.canReuseRows(oldRows, newRows))
    }

    @Test func canReorderWhenIdentitiesMatchButOrderChanges() {
        let oldRows = [itemRow(id: "a", flatIndex: 0), itemRow(id: "b", flatIndex: 1)]
        let newRows = [itemRow(id: "b", flatIndex: 0), itemRow(id: "a", flatIndex: 1)]
        #expect(LauncherListRowReuse.canReorderRows(oldRows, newRows))
    }

    @Test func cannotReuseWhenListNestChanges() {
        let oldRows = [itemRow(id: "a", flatIndex: 0)]
        let nested = ResultItem(
            id: ResultID(module: .apps, key: "a"),
            title: "App",
            titleAttributed: AttributedString("App"),
            subtitle: nil,
            icon: .symbol("app"),
            primaryAction: Action(id: ActionID(module: .apps, key: "a"), title: "App", kind: .noop),
            rankingHints: RankingHints(basePriority: 0),
            listNest: .child(isLast: true)
        )
        let newRows = [LauncherListRows.Row(kind: .item(nested, flatIndex: 0))]
        #expect(!LauncherListRowReuse.canReuseRows(oldRows, newRows))
    }
}
