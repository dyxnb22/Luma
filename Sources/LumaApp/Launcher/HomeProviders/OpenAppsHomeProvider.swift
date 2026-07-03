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
    static let defaultAppLimit = 8

    private let appActivationTracker: AppActivationTracker
    private var cachedSnapshots: [AppRuntimeSnapshot] = []
    private var appLimit: Int? = defaultAppLimit
    private var collapsedBundleIDs = Set<String>()
    private var refreshTask: Task<Void, Never>?
    private var isActive = false

    init(appActivationTracker: AppActivationTracker) {
        self.appActivationTracker = appActivationTracker
    }

    func setActive(_ active: Bool) {
        guard isActive != active else { return }
        isActive = active
        if active {
            refreshTask?.cancel()
            refreshTask = Task { [weak self] in await self?.refreshLoop() }
        } else {
            refreshTask?.cancel()
            refreshTask = nil
        }
    }

    func configure(appLimit: Int?, collapsedBundleIDs: Set<String>) {
        self.appLimit = appLimit
        self.collapsedBundleIDs = collapsedBundleIDs
    }

    func invalidateCache() {
        Task { [weak self] in await self?.refreshOnce() }
    }

    func items() async -> [ResultItem] {
        if cachedSnapshots.isEmpty {
            await refreshOnce()
        }
        let snapshots = cachedSnapshots
        guard !snapshots.isEmpty else { return [] }

        let bundleIDs = snapshots.map(\.bundleID)
        let rankedIDs = await appActivationTracker.rankedBundleIDs(from: bundleIDs)
        let byID = Dictionary(uniqueKeysWithValues: snapshots.map { ($0.bundleID, $0) })
        let ordered = rankedIDs.compactMap { byID[$0] }

        let limit = appLimit ?? ordered.count
        let visible = Array(ordered.prefix(limit))

        var items: [ResultItem] = []
        for app in visible {
            if app.windows.count > 1 {
                let isExpanded = !collapsedBundleIDs.contains(app.bundleID)
                items.append(Self.expandableAppRow(for: app, isExpanded: isExpanded))
                if isExpanded {
                    items.append(contentsOf: app.windows.enumerated().map { index, window in
                        Self.windowRow(for: window, app: app, isLast: index == app.windows.count - 1)
                    })
                }
            } else {
                items.append(Self.appRow(for: app))
            }
        }
        return items
    }

    private func refreshLoop() async {
        await refreshOnce()
        while !Task.isCancelled {
            try? await Task.sleep(for: .seconds(2))
            if Task.isCancelled { break }
            await refreshOnce()
        }
    }

    private func refreshOnce() async {
        let snapshots = await MainActor.run { Self.collectRunningApps() }
        cachedSnapshots = snapshots
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

    private static func secondaryActions(for app: AppRuntimeSnapshot) -> [Action] {
        var secondary: [Action] = []
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
        return secondary
    }

    private static func appRow(for app: AppRuntimeSnapshot) -> ResultItem {
        let snapshot = RunningAppSnapshot(
            bundleID: app.bundleID,
            name: app.name,
            appPath: app.appURL.path,
            windowCount: app.windowCount
        )
        return OpenAppsResultBuilder.resultItem(for: snapshot, secondaryActions: secondaryActions(for: app))
    }

    private static func expandableAppRow(for app: AppRuntimeSnapshot, isExpanded: Bool) -> ResultItem {
        let snapshot = RunningAppSnapshot(
            bundleID: app.bundleID,
            name: app.name,
            appPath: app.appURL.path,
            windowCount: app.windowCount
        )
        return OpenAppsResultBuilder.expandableResultItem(
            for: snapshot,
            isExpanded: isExpanded,
            secondaryActions: secondaryActions(for: app)
        )
    }

    private static func windowRow(
        for window: OpenWindowSnapshot,
        app: AppRuntimeSnapshot,
        isLast: Bool
    ) -> ResultItem {
        let displayTitle = IDEWindowTitle.sidebarLabel(
            rawTitle: window.title,
            bundleID: app.bundleID,
            appName: app.name
        )
        return OpenAppsResultBuilder.windowRow(
            for: RunningWindowSnapshot(
                bundleID: app.bundleID,
                appName: app.name,
                windowID: window.windowID,
                pid: window.pid,
                title: displayTitle,
                axTitle: window.title,
                bounds: window.bounds,
                isMain: window.isMain,
                isMinimized: window.isMinimized
            ),
            listNest: .child(isLast: isLast)
        )
    }
}
