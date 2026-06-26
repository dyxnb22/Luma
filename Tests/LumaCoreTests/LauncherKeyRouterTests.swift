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

@Test func keyRouterDismissesActionPanelOnTabWhenVisible() {
    let outcome = LauncherKeyRouter.route(
        command: .tab,
        mode: .results,
        itemCount: 3,
        actionPanelVisible: true
    )
    #expect(outcome == .dismissActionPanel)
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

@Test func resolveRunUsesOpenModuleDetailAction() {
    let todo = ModuleIdentifier(rawValue: "luma.todo")
    let item = ResultItem(
        id: ResultID(module: todo, key: "contextual.today"),
        title: "Todos",
        titleAttributed: "Todos",
        icon: .none,
        primaryAction: Action(
            id: ActionID(module: todo, key: "open"),
            title: "Open",
            kind: .openModuleDetail(todo, payload: nil)
        ),
        rankingHints: RankingHints()
    )
    #expect(LauncherKeyRouter.resolveRun(item: item) == .runItem(item))
}

@Test func actionKeepsLauncherVisibleForInPanelIntents() {
    let wordbook = ModuleIdentifier(rawValue: "luma.wordbook")
    let openDetail = Action(
        id: ActionID(module: wordbook, key: "open"),
        title: "Open",
        kind: .openModuleDetail(wordbook, payload: nil)
    )
    #expect(openDetail.keepsLauncherVisible)

    let apps = ModuleIdentifier(rawValue: "luma.apps")
    let launch = Action(
        id: ActionID(module: apps, key: "launch"),
        title: "Launch",
        kind: .launchApp(URL(fileURLWithPath: "/Applications"))
    )
    #expect(!launch.keepsLauncherVisible)
}

@Test func replaceQueryActionKeepsLauncherVisible() {
    let action = Action(
        id: ActionID(module: .commandEntry, key: "replace"),
        title: "Use apps",
        kind: .replaceQuery("apps ")
    )
    #expect(action.keepsLauncherVisible)
}
