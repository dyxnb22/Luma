import Foundation
import LumaCore
import LumaModules
import LumaServices

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider
    private var collapsedAppBundleIDs = Set<String>()

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
    }

    func setActive(_ active: Bool) async {
        await ClipboardPasteboardCache.shared.setActive(active)
        if active {
            // Panel signals are not first-frame critical; avoid competing with home render.
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
    }

    func hasExpandedOpenApps() -> Bool {
        !collapsedAppBundleIDs.isEmpty
    }

    func snapshot() async -> LauncherHomeSnapshot {
        await openApps.configure(collapsedBundleIDs: collapsedAppBundleIDs)
        return await aggregator.snapshot()
    }
}
