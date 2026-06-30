import Foundation
import LumaCore
import LumaServices

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider
    private var contextual: ContextualHomeProvider
    private var showAllApps = false
    private var collapsedAppBundleIDs = Set<String>()

    init(
        openApps: OpenAppsHomeProvider,
        recentActions: RecentActionsHomeProvider = RecentActionsHomeProvider(),
        resume: ResumeHomeProvider = ResumeHomeProvider(),
        contextual: ContextualHomeProvider,
        setup: SetupHomeProvider? = nil
    ) {
        self.openApps = openApps
        self.contextual = contextual
        self.aggregator = LauncherHomeAggregator(
            openApps: openApps,
            recentActions: recentActions,
            resume: resume,
            contextual: contextual,
            setup: setup
        )
    }

    func updatePinnedModuleIDs(_ ids: Set<ModuleIdentifier>) {
        contextual.updatePinnedModuleIDs(ids)
    }

    func updateEnabledModuleIDs(_ ids: Set<ModuleIdentifier>) {
        contextual.updateEnabledModuleIDs(ids)
    }

    func expandAllApps() {
        showAllApps = true
    }

    func resetExpansion() {
        showAllApps = false
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
        await openApps.configure(
            appLimit: showAllApps ? nil : OpenAppsHomeProvider.defaultAppLimit,
            collapsedBundleIDs: collapsedAppBundleIDs
        )
        return await aggregator.snapshot()
    }
}
