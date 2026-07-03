import Foundation
import LumaCore
import LumaModules
import LumaServices

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider
    private let enablementGate: HomeEnablementGate
    private var collapsedAppBundleIDs = Set<String>()

    init(
        openApps: OpenAppsHomeProvider,
        enablementGate: HomeEnablementGate
    ) {
        self.openApps = openApps
        self.enablementGate = enablementGate
        self.aggregator = LauncherHomeAggregator(openApps: openApps)
    }

    func updateEnabledModuleIDs(_ ids: Set<ModuleIdentifier>) async {
        enablementGate.update(ids)
    }

    func resetExpansion() {
        collapsedAppBundleIDs.removeAll()
    }

    func setActive(_ active: Bool) async {
        await openApps.setActive(active)
        await ClipboardPasteboardCache.shared.setActive(active)
        if active {
            _ = await CurrentProjectService.shared.snapshot()
            _ = await SelectionSnapshotService.shared.snapshot()
        }
    }

    func toggleAppWindows(bundleID: String) {
        if collapsedAppBundleIDs.contains(bundleID) {
            collapsedAppBundleIDs.remove(bundleID)
        } else {
            collapsedAppBundleIDs.insert(bundleID)
        }
    }

    func snapshot() async -> LauncherHomeSnapshot {
        await openApps.configure(collapsedBundleIDs: collapsedAppBundleIDs)
        return await aggregator.snapshot()
    }
}
