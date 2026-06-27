import Foundation
import LumaCore
import LumaServices

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider
    private var showAllApps = false
    private var expandedAppBundleIDs = Set<String>()

    init(
        openApps: OpenAppsHomeProvider,
        recentActions: RecentActionsHomeProvider = RecentActionsHomeProvider(),
        resume: ResumeHomeProvider = ResumeHomeProvider(),
        contextual: ContextualHomeProvider
    ) {
        self.openApps = openApps
        self.aggregator = LauncherHomeAggregator(
            openApps: openApps,
            recentActions: recentActions,
            resume: resume,
            contextual: contextual
        )
    }

    func expandAllApps() {
        showAllApps = true
    }

    func resetExpansion() {
        showAllApps = false
        expandedAppBundleIDs.removeAll()
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
        if expandedAppBundleIDs.contains(bundleID) {
            expandedAppBundleIDs.remove(bundleID)
        } else {
            expandedAppBundleIDs.insert(bundleID)
        }
    }

    func snapshot() async -> LauncherHomeSnapshot {
        await openApps.configure(
            appLimit: showAllApps ? nil : OpenAppsHomeProvider.defaultAppLimit,
            expandedBundleIDs: expandedAppBundleIDs
        )
        return await aggregator.snapshot()
    }
}
