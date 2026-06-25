import AppKit
import Foundation
import LumaCore
import LumaModules
import LumaServices

private struct AppRuntimeSnapshot: Sendable {
    let bundleID: String
    let name: String
    let appURL: URL
    let windowCount: Int
    let windows: [OpenWindowSnapshot]
}

actor OpenAppsHomeProvider: LauncherHomeProvider {
    private let appActivationTracker: AppActivationTracker
    private var showsAll = false
    private static let collapsedLimit = 6

    init(appActivationTracker: AppActivationTracker) {
        self.appActivationTracker = appActivationTracker
    }

    func resetExpanded() {
        showsAll = false
    }

    func expandAll() {
        showsAll = true
    }

    func items() async -> [ResultItem] {
        let snapshots = await MainActor.run { Self.collectRunningApps() }
        guard !snapshots.isEmpty else { return [] }

        let bundleIDs = snapshots.map(\.bundleID)
        let rankedIDs = await appActivationTracker.rankedBundleIDs(from: bundleIDs)
        let byID = Dictionary(uniqueKeysWithValues: snapshots.map { ($0.bundleID, $0) })
        let ordered = rankedIDs.compactMap { byID[$0] }

        let visible = showsAll ? ordered : Array(ordered.prefix(Self.collapsedLimit))
        var items = visible.map { Self.resultItem(for: $0) }

        if !showsAll, ordered.count > Self.collapsedLimit {
            items.append(OpenAppsResultBuilder.moreRow(hiddenCount: ordered.count - Self.collapsedLimit))
        }
        return items
    }

    @MainActor
    private static func collectRunningApps() -> [AppRuntimeSnapshot] {
        let selfPID = ProcessInfo.processInfo.processIdentifier
        let axGranted = AXService.isProcessTrusted()
        let runningApps = NSWorkspace.shared.runningApplications.filter { app in
            app.activationPolicy == .regular
                && app.bundleIdentifier != nil
                && app.processIdentifier != selfPID
        }
        var seenIDs = Set<String>()
        var snapshots: [AppRuntimeSnapshot] = []
        for app in runningApps {
            guard let bundleID = app.bundleIdentifier, seenIDs.insert(bundleID).inserted else { continue }
            let name = app.localizedName ?? bundleID
            let windows: [OpenWindowSnapshot] = axGranted
                ? AXService.enumerateWindows(for: app.processIdentifier, appName: name)
                : []
            let url = app.bundleURL ?? NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID)
                ?? URL(fileURLWithPath: "/Applications")
            snapshots.append(AppRuntimeSnapshot(
                bundleID: bundleID,
                name: name,
                appURL: url,
                windowCount: max(windows.count, 1),
                windows: windows
            ))
        }
        return snapshots
    }

    private static func resultItem(for app: AppRuntimeSnapshot) -> ResultItem {
        var secondary: [Action] = []
        if app.windows.count > 1 {
            for window in app.windows {
                secondary.append(Action(
                    id: ActionID(module: .windows, key: "focus.\(window.id)"),
                    title: "Focus — \(window.title)",
                    kind: .focusWindow(windowID: window.windowID, pid: window.pid, title: window.title)
                ))
            }
        }
        if let quitPayload = try? ModuleActionCoding.encode(AppsAction.quit(bundleID: app.bundleID)) {
            secondary.append(Action(
                id: ActionID(module: .apps, key: "quit.\(app.bundleID)"),
                title: "Quit",
                kind: .custom(payload: quitPayload, handler: .apps)
            ))
        }
        secondary.append(Action(
            id: ActionID(module: .apps, key: "copyPath.\(app.bundleID)"),
            title: "Copy app path",
            kind: .copyToPasteboard(app.appURL.path)
        ))

        let snapshot = RunningAppSnapshot(
            bundleID: app.bundleID,
            name: app.name,
            appPath: app.appURL.path,
            windowCount: app.windowCount
        )
        return OpenAppsResultBuilder.resultItem(for: snapshot, secondaryActions: secondary)
    }
}
