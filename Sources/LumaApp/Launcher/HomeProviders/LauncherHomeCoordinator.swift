import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider
    private var collapsedAppBundleIDs = Set<String>()
    private var cachedSnapshot: LauncherHomeSnapshot?
    private var snapshotGeneration: UInt64 = 0

    init(
        openApps: OpenAppsHomeProvider,
        onHomeDataUpdated: (@Sendable () -> Void)? = nil
    ) {
        self.openApps = openApps
        self.aggregator = LauncherHomeAggregator(openApps: openApps)
        if let onHomeDataUpdated {
            Task { await openApps.setOnCacheUpdated(onHomeDataUpdated) }
        }
    }

    func resetExpansion() {
        collapsedAppBundleIDs.removeAll()
        invalidateSnapshotCache()
    }

    func setActive(_ active: Bool) async {
        await ClipboardPasteboardCache.shared.setActive(active)
        await openApps.setActive(active)
        if active {
            Task.detached(priority: .utility) {
                _ = await CurrentProjectService.shared.snapshot()
                _ = await SelectionSnapshotService.shared.snapshot()
            }
        }
    }

    func toggleAppWindows(bundleID: String) {
        if collapsedAppBundleIDs.contains(bundleID) {
            collapsedAppBundleIDs.remove(bundleID)
        } else {
            collapsedAppBundleIDs.insert(bundleID)
        }
        invalidateSnapshotCache()
    }

    func hasToggledOpenAppWindows() -> Bool {
        !collapsedAppBundleIDs.isEmpty
    }

    func cachedSnapshotIfAvailable() -> LauncherHomeSnapshot? {
        cachedSnapshot
    }

    func currentSnapshotGeneration() -> UInt64 {
        snapshotGeneration
    }

    func invalidateSnapshotCache() {
        cachedSnapshot = nil
    }

    func snapshot(forceRefresh: Bool = false) async -> LauncherHomeSnapshot {
        if !forceRefresh, let cachedSnapshot {
            return cachedSnapshot
        }
        LauncherPerfCounters.increment(.homeSnapshot)
        await openApps.configure(collapsedBundleIDs: collapsedAppBundleIDs)
        let snapshot = await aggregator.snapshot()
        cachedSnapshot = snapshot
        snapshotGeneration &+= 1
        return snapshot
    }

    func revalidateSnapshotInBackground() async {
        _ = await snapshot(forceRefresh: true)
    }
}
