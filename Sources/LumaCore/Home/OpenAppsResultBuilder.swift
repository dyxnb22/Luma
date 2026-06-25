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

public enum OpenAppsResultBuilder {
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
