import Foundation
import CoreGraphics

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
    public let axTitle: String
    public let bounds: CGRect
    public let isMain: Bool
    public let isMinimized: Bool

    public init(
        bundleID: String,
        appName: String,
        windowID: UInt32,
        pid: Int32,
        title: String,
        axTitle: String,
        bounds: CGRect = .zero,
        isMain: Bool,
        isMinimized: Bool
    ) {
        self.bundleID = bundleID
        self.appName = appName
        self.windowID = windowID
        self.pid = pid
        self.title = title
        self.axTitle = axTitle
        self.bounds = bounds
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
        let windowKey = Self.windowIdentityKey(windowID: window.windowID, axTitle: window.axTitle)
        let bounds = window.bounds == .zero ? nil : WindowBounds(window.bounds)
        return ResultItem(
            id: ResultID(module: windowsModule, key: "openApps.window.\(window.pid).\(windowKey)"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: state,
            icon: .symbol("macwindow"),
            primaryAction: Action(
                id: ActionID(module: windowsModule, key: "focus.\(window.pid).\(windowKey)"),
                title: "Focus Window",
                kind: .focusWindow(
                    windowID: window.windowID,
                    pid: window.pid,
                    title: title,
                    axTitle: window.axTitle,
                    bounds: bounds
                )
            ),
            rankingHints: RankingHints(basePriority: 99),
            displayDensity: .compact,
            listNest: listNest
        )
    }

    private static func windowIdentityKey(windowID: UInt32, axTitle: String) -> String {
        if windowID != 0 { return String(windowID) }
        let slug = axTitle
            .lowercased()
            .replacingOccurrences(of: " ", with: "-")
            .filter { $0.isLetter || $0.isNumber || $0 == "-" }
        return slug.isEmpty ? "untitled" : String(slug.prefix(48))
    }
}
