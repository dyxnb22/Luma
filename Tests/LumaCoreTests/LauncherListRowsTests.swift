import Foundation
import Testing
import LumaCore

@Test func openAppsBuilderCreatesLaunchAction() {
    let item = OpenAppsResultBuilder.resultItem(
        for: RunningAppSnapshot(
            bundleID: "com.apple.Safari",
            name: "Safari",
            appPath: "/Applications/Safari.app",
            windowCount: 1
        ),
        secondaryActions: []
    )
    #expect(item.title == "Safari")
    #expect(item.id.key == "open.com.apple.Safari")
    if case .launchApp(let url) = item.primaryAction.kind {
        #expect(url.path == "/Applications/Safari.app")
    } else {
        Issue.record("Expected launchApp action")
    }
}

@Test func openAppsBuilderShowsWindowCountSubtitle() {
    let item = OpenAppsResultBuilder.resultItem(
        for: RunningAppSnapshot(
            bundleID: "com.apple.Xcode",
            name: "Xcode",
            appPath: "/Applications/Xcode.app",
            windowCount: 3
        ),
        secondaryActions: [Action(
            id: ActionID(module: ModuleIdentifier(rawValue: "luma.windows"), key: "f"),
            title: "Focus",
            kind: .noop
        )]
    )
    #expect(item.subtitle == "3 windows")
    #expect(item.secondaryActions.count == 1)
}

@Test func openAppsBuilderCreatesExpandableMultiWindowRow() {
    let item = OpenAppsResultBuilder.expandableResultItem(
        for: RunningAppSnapshot(
            bundleID: "com.todesktop.230313mzl4w4u92",
            name: "Cursor",
            appPath: "/Applications/Cursor.app",
            windowCount: 3
        ),
        isExpanded: false,
        secondaryActions: []
    )
    #expect(item.title == "Cursor")
    #expect(item.subtitle == "3 windows")
    #expect(item.id.key == "openApps.windows.toggle.com.todesktop.230313mzl4w4u92")
    #expect(item.primaryAction.title == "Show Windows")
}

@Test func openAppsBuilderCreatesWindowFocusRow() {
    let item = OpenAppsResultBuilder.windowRow(for: RunningWindowSnapshot(
        bundleID: "com.todesktop.230313mzl4w4u92",
        appName: "Cursor",
        windowID: 42,
        pid: 123,
        title: "Luma — Sources",
        axTitle: "Luma — Sources",
        isMain: true,
        isMinimized: false
    ))
    #expect(item.title == "Luma — Sources")
    #expect(item.subtitle == "focused")
    #expect(item.listNest == .child(isLast: true))
    if case .focusWindow(let windowID, let pid, let title, _, _) = item.primaryAction.kind {
        #expect(windowID == 42)
        #expect(pid == 123)
        #expect(title == "Luma — Sources")
    } else {
        Issue.record("Expected focusWindow action")
    }
}

@Test func listRowsMapHomeSectionsWithGlobalShortcutIndex() {
    let appsID = ModuleIdentifier(rawValue: "luma.apps")
    let todoID = ModuleIdentifier(rawValue: "luma.todo")
    let snapshot = LauncherHomeSnapshot(sections: [
        LauncherHomeSection(kind: .openApps, items: [
            ResultItem(
                id: ResultID(module: appsID, key: "a"),
                title: "A",
                titleAttributed: "A",
                icon: .none,
                primaryAction: Action(id: ActionID(module: appsID, key: "a"), title: "Open", kind: .noop),
                rankingHints: RankingHints()
            )
        ]),
        LauncherHomeSection(kind: .suggested, items: [
            ResultItem(
                id: ResultID(module: todoID, key: "b"),
                title: "B",
                titleAttributed: "B",
                icon: .none,
                primaryAction: Action(id: ActionID(module: todoID, key: "b"), title: "Open", kind: .noop),
                rankingHints: RankingHints()
            )
        ])
    ])
    let rows = LauncherListRows.rows(for: snapshot)
    #expect(rows.count == 4)
    if case .sectionHeader(let title, let shortcut) = rows[0].kind {
        #expect(title == "OPEN APPS")
        #expect(shortcut == 1)
    } else {
        Issue.record("Expected section header")
    }
    if case .sectionHeader(let suggestedTitle, let suggestedShortcut) = rows[2].kind {
        #expect(suggestedTitle == "SUGGESTED")
        #expect(suggestedShortcut == 2)
    } else {
        Issue.record("Expected SUGGESTED section header")
    }
    if case .item(_, let index) = rows[3].kind {
        #expect(index == 1)
    } else {
        Issue.record("Expected second item at flat index 1")
    }
}
