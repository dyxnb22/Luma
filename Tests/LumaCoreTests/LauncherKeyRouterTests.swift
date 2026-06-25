import Foundation
import Testing
import LumaCore

@Test func keyRouterOpensActionPanelOnTab() {
    let outcome = LauncherKeyRouter.route(
        command: .tab,
        mode: .home,
        itemCount: 3,
        actionPanelVisible: false
    )
    #expect(outcome == .openActionPanel)
}

@Test func keyRouterIgnoresTabInDetailMode() {
    let outcome = LauncherKeyRouter.route(
        command: .tab,
        mode: .detail,
        itemCount: 3,
        actionPanelVisible: false
    )
    #expect(outcome == .handled)
}

@Test func keyRouterCommandNumberJumpsToFlatIndex() {
    let outcome = LauncherKeyRouter.route(
        command: .commandNumber(2),
        mode: .results,
        itemCount: 5,
        actionPanelVisible: false
    )
    #expect(outcome == .jumpToFlatIndex(1))
}

@Test func resolveRunDetectsOpenAppsMoreRow() {
    let item = OpenAppsResultBuilder.moreRow(hiddenCount: 4)
    #expect(LauncherKeyRouter.resolveRun(item: item) == .expandOpenApps)
}

@Test func resolveRunDetectsContextualTodo() {
    let item = ResultItem(
        id: ResultID(module: ModuleIdentifier(rawValue: "luma.todo"), key: "contextual.today"),
        title: "Todos",
        titleAttributed: "Todos",
        icon: .none,
        primaryAction: Action(
            id: ActionID(module: ModuleIdentifier(rawValue: "luma.todo"), key: "x"),
            title: "Open",
            kind: .noop
        ),
        rankingHints: RankingHints()
    )
    #expect(LauncherKeyRouter.resolveRun(item: item) == .openTodoDetail)
}
