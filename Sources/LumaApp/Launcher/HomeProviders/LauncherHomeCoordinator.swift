import Foundation
import LumaCore

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider

    init(
        openApps: OpenAppsHomeProvider,
        recent: RecentActionsHomeProvider,
        contextual: ContextualHomeProvider
    ) {
        self.openApps = openApps
        self.aggregator = LauncherHomeAggregator(
            openApps: openApps,
            recent: recent,
            contextual: contextual
        )
    }

    func resetOpenAppsExpansion() async {
        await openApps.resetExpanded()
    }

    func expandOpenApps() async {
        await openApps.expandAll()
    }

    func snapshot() async -> LauncherHomeSnapshot {
        await aggregator.snapshot()
    }
}
