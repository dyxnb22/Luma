import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

private struct AppRuntimeSnapshot: Sendable, Equatable {
    let bundleID: String
    let name: String
    let appURL: URL
    let windowCount: Int
    let windows: [OpenWindowSnapshot]

    static func == (lhs: AppRuntimeSnapshot, rhs: AppRuntimeSnapshot) -> Bool {
        lhs.bundleID == rhs.bundleID
            && lhs.name == rhs.name
            && lhs.appURL == rhs.appURL
            && lhs.windowCount == rhs.windowCount
            && lhs.windows.stableOpenAppsIdentity == rhs.windows.stableOpenAppsIdentity
    }
}

private extension Array where Element == OpenWindowSnapshot {
    var stableOpenAppsIdentity: [StableOpenAppsWindowIdentity] {
        map(StableOpenAppsWindowIdentity.init)
    }
}

private struct StableOpenAppsWindowIdentity: Equatable {
    let windowID: UInt32
    let pid: Int32
    let title: String
    let isMinimized: Bool

    init(_ window: OpenWindowSnapshot) {
        self.windowID = window.windowID
        self.pid = window.pid
        self.title = window.title
        self.isMinimized = window.isMinimized
    }
}

actor OpenAppsHomeProvider: LauncherHomeProvider {
    private let appActivationTracker: AppActivationTracker
    private let windowEnumerator: any AXWindowEnumerating
    private var cachedSnapshots: [AppRuntimeSnapshot] = []
    private var collapsedBundleIDs = Set<String>()
    private var refreshTask: Task<Void, Never>?
    private var skeletonRefreshTask: Task<Void, Never>?
    private var isActive = false
    private var onCacheUpdated: (@Sendable () -> Void)?

    init(
        appActivationTracker: AppActivationTracker,
        windowEnumerator: any AXWindowEnumerating = LiveAXWindowEnumerator()
    ) {
        self.appActivationTracker = appActivationTracker
        self.windowEnumerator = windowEnumerator
    }

    func setOnCacheUpdated(_ handler: (@Sendable () -> Void)?) {
        onCacheUpdated = handler
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
            skeletonRefreshTask?.cancel()
            skeletonRefreshTask = nil
        }
    }

    func configure(collapsedBundleIDs: Set<String>) {
        self.collapsedBundleIDs = collapsedBundleIDs
    }

    func items() async -> [ResultItem] {
        if cachedSnapshots.isEmpty {
            // NSWorkspace.runningApplications is MainActor-isolated; AX/CGWindow IPC is offloaded in refreshOnce.
            let metadata = await Self.collectAppMetadata()
            scheduleFullRefreshIfNeeded()
            guard !metadata.isEmpty else { return [] }
            return await buildItems(from: metadata.map(Self.skeletonSnapshot(from:)))
        }
        return await buildItems(from: cachedSnapshots)
    }

    func isWarming() async -> Bool {
        isActive && cachedSnapshots.isEmpty && skeletonRefreshTask != nil
    }

    private func refreshLoop() async {
        if cachedSnapshots.isEmpty {
            await refreshOnce()
        }
        while !Task.isCancelled {
            try? await Task.sleep(for: .seconds(2))
            if Task.isCancelled { break }
            await refreshOnce()
        }
    }

    private func scheduleFullRefreshIfNeeded() {
        guard skeletonRefreshTask == nil else { return }
        skeletonRefreshTask = Task { [weak self] in
            await self?.refreshOnce()
            await self?.clearSkeletonRefreshTask()
        }
    }

    private func clearSkeletonRefreshTask() {
        skeletonRefreshTask = nil
    }

    private func refreshOnce() async {
        LauncherPerfCounters.increment(.openAppsRefresh)
        let metadata = await Self.collectAppMetadata()
        guard !metadata.isEmpty else {
            let hadCache = !cachedSnapshots.isEmpty
            cachedSnapshots = []
            if hadCache { onCacheUpdated?() }
            return
        }

        // AX + CGWindow IPC must not run on MainActor — it blocks hotkey→visible (ADR-023 ≤4ms).
        let windowsByPID = await Task.detached { [windowEnumerator] in
            RunningAppsWindowCollector.windowsByPID(for: metadata, using: windowEnumerator)
        }.value

        let snapshots = metadata.map { app in
            let windows = windowsByPID[app.pid] ?? []
            return Self.snapshot(from: app, windows: windows)
        }
        let changed = snapshots != cachedSnapshots
        cachedSnapshots = snapshots
        if changed { onCacheUpdated?() }
    }

    @MainActor
    private static func collectAppMetadata() -> [RunningAppMetadata] {
        let selfPID = ProcessInfo.processInfo.processIdentifier
        let runningApps = NSWorkspace.shared.runningApplications.filter { app in
            app.activationPolicy == .regular
                && app.bundleIdentifier != nil
                && app.processIdentifier != selfPID
        }
        var seenIDs = Set<String>()
        var metadata: [RunningAppMetadata] = []
        for app in runningApps {
            guard let bundleID = app.bundleIdentifier, seenIDs.insert(bundleID).inserted else { continue }
            let name = app.localizedName ?? bundleID
            let url = app.bundleURL ?? URL(fileURLWithPath: "/Applications")
            metadata.append(RunningAppMetadata(
                pid: app.processIdentifier,
                bundleID: bundleID,
                name: name,
                appURLPath: url.path
            ))
        }
        return metadata
    }

    private static func skeletonSnapshot(from metadata: RunningAppMetadata) -> AppRuntimeSnapshot {
        snapshot(from: metadata, windows: [])
    }

    private static func snapshot(from metadata: RunningAppMetadata, windows: [OpenWindowSnapshot]) -> AppRuntimeSnapshot {
        AppRuntimeSnapshot(
            bundleID: metadata.bundleID,
            name: metadata.name,
            appURL: URL(fileURLWithPath: metadata.appURLPath),
            windowCount: max(windows.count, 1),
            windows: windows
        )
    }

    private func buildItems(from snapshots: [AppRuntimeSnapshot]) async -> [ResultItem] {
        guard !snapshots.isEmpty else { return [] }

        let bundleIDs = snapshots.map(\.bundleID)
        let rankedIDs = await appActivationTracker.rankedBundleIDs(from: bundleIDs)
        let byID = Dictionary(uniqueKeysWithValues: snapshots.map { ($0.bundleID, $0) })
        let ordered = rankedIDs.compactMap { byID[$0] }

        var items: [ResultItem] = []
        for app in ordered {
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
