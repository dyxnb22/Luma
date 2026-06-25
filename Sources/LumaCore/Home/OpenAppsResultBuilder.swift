import Foundation

public struct RunningAppSnapshot: Sendable, Equatable {
    public let bundleID: String
    public let name: String
    public let appPath: String
    public let windowCount: Int

    public init(bundleID: String, name: String, appPath: String, windowCount: Int) {
        self.bundleID = bundleID
        self.name = name
        self.appPath = appPath
        self.windowCount = windowCount
    }
}

public struct RunningWindowSnapshot: Sendable, Equatable {
    public let bundleID: String
    public let appName: String
    public let windowID: UInt32
    public let pid: Int32
    public let title: String
    public let isMain: Bool
    public let isMinimized: Bool

    public init(
        bundleID: String,
        appName: String,
        windowID: UInt32,
        pid: Int32,
        title: String,
        isMain: Bool,
        isMinimized: Bool
    ) {
        self.bundleID = bundleID
        self.appName = appName
        self.windowID = windowID
        self.pid = pid
        self.title = title
        self.isMain = isMain
        self.isMinimized = isMinimized
    }
}

public enum OpenAppsResultBuilder {
    public static let toggleWindowsKeyPrefix = "openApps.windows.toggle."

    public static func resultItem(for app: RunningAppSnapshot, secondaryActions: [Action]) -> ResultItem {
        let url = URL(fileURLWithPath: app.appPath)
        let appsModule = ModuleIdentifier(rawValue: "luma.apps")
        let subtitle: String? = app.windowCount > 1 ? "\(app.windowCount) windows" : nil
        return ResultItem(
            id: ResultID(module: appsModule, key: "open.\(app.bundleID)"),
            title: app.name,
            titleAttributed: AttributedString(app.name),
            subtitle: subtitle,
            icon: .bundleID(app.bundleID),
            primaryAction: Action(
                id: ActionID(module: appsModule, key: "activate.\(app.bundleID)"),
                title: "Open",
                kind: .launchApp(url)
            ),
            secondaryActions: secondaryActions,
            rankingHints: RankingHints(basePriority: 100)
        )
    }

    public static func expandableResultItem(
        for app: RunningAppSnapshot,
        isExpanded: Bool,
        secondaryActions: [Action]
    ) -> ResultItem {
        let appsModule = ModuleIdentifier(rawValue: "luma.apps")
        let subtitle = isExpanded ? "\(app.windowCount) windows expanded" : "\(app.windowCount) windows"
        return ResultItem(
            id: ResultID(module: appsModule, key: "\(toggleWindowsKeyPrefix)\(app.bundleID)"),
            title: app.name,
            titleAttributed: AttributedString(app.name),
            subtitle: subtitle,
            icon: .bundleID(app.bundleID),
            primaryAction: Action(
                id: ActionID(module: appsModule, key: "\(toggleWindowsKeyPrefix)\(app.bundleID)"),
                title: isExpanded ? "Hide Windows" : "Show Windows",
                kind: .noop
            ),
            secondaryActions: secondaryActions,
            rankingHints: RankingHints(basePriority: 100)
        )
    }

    public static func windowRow(
        for window: RunningWindowSnapshot,
        listNest: ResultListNest = .child(isLast: true)
    ) -> ResultItem {
        let windowsModule = ModuleIdentifier(rawValue: "luma.windows")
        let title = window.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            ? window.appName
            : window.title
        let state = window.isMinimized ? "minimized" : (window.isMain ? "focused" : "window")
        return ResultItem(
            id: ResultID(module: windowsModule, key: "openApps.window.\(window.pid).\(window.windowID)"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: state,
            icon: .symbol("macwindow"),
            primaryAction: Action(
                id: ActionID(module: windowsModule, key: "focus.\(window.pid).\(window.windowID)"),
                title: "Focus Window",
                kind: .focusWindow(windowID: window.windowID, pid: window.pid, title: window.title)
            ),
            rankingHints: RankingHints(basePriority: 99),
            displayDensity: .compact,
            listNest: listNest
        )
    }

    public static func moreRow(hiddenCount: Int) -> ResultItem {
        let appsModule = ModuleIdentifier(rawValue: "luma.apps")
        return ResultItem(
            id: ResultID(module: appsModule, key: "openApps.more"),
            title: "+\(hiddenCount) more",
            titleAttributed: AttributedString("+\(hiddenCount) more"),
            subtitle: nil,
            icon: .symbol("ellipsis.circle"),
            primaryAction: Action(
                id: ActionID(module: appsModule, key: "openApps.expand"),
                title: "Show all",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: 0)
        )
    }
}
