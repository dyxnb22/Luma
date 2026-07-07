import CoreGraphics
import Foundation
import LumaCore

public actor WindowsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .windows,
        displayName: "Windows",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false, // Deferred: not in ModuleRegistry.allBundles; BuiltInModules.makeDeferred() only
        priority: 3,
        queryTimeout: .milliseconds(60)
    )

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let windows = WindowEnumerator.windows()
        let matches = windows
            .filter { query.normalized.isEmpty || $0.title.lowercased().contains(query.normalized) || $0.appName.lowercased().contains(query.normalized) }
            .prefix(20)
            .map(result)
        return ModuleResult(items: Array(matches))
    }

    private func result(_ window: WindowRecord) -> ResultItem {
        let title = window.title.isEmpty ? window.appName : window.title
        let id = ResultID(module: Self.manifest.identifier, key: "\(window.windowID)|\(title)")
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: window.appName,
            icon: .symbol("macwindow"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "focus.\(window.windowID)"),
                title: "Focus Window",
                kind: .focusWindow(windowID: window.windowID, pid: window.pid, title: window.title, axTitle: nil, bounds: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}

public struct WindowRecord: Sendable, Hashable {
    public let windowID: UInt32
    public let pid: Int32
    public let appName: String
    public let title: String
}

public enum WindowEnumerator {
    public static func windows() -> [WindowRecord] {
        guard let info = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else {
            return []
        }

        let myPID = ProcessInfo.processInfo.processIdentifier
        return info.compactMap { item in
            guard let id = item[kCGWindowNumber as String] as? UInt32,
                  let pid = item[kCGWindowOwnerPID as String] as? Int32,
                  let owner = item[kCGWindowOwnerName as String] as? String else { return nil }
            guard pid != myPID else { return nil }
            let title = item[kCGWindowName as String] as? String ?? ""
            guard !owner.isEmpty else { return nil }
            return WindowRecord(windowID: id, pid: pid, appName: owner, title: title)
        }
    }
}
