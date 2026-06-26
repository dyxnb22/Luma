import Foundation
import LumaCore

/// App-layer wiring for home list providers and open-apps expansion.
actor LauncherHomeCoordinator {
    private let aggregator: LauncherHomeAggregator
    private let openApps: OpenAppsHomeProvider

    init(
        openApps: OpenAppsHomeProvider,
        contextual: ContextualHomeProvider
    ) {
        self.openApps = openApps
        self.aggregator = LauncherHomeAggregator(
            openApps: openApps,
            contextual: contextual
        )
    }


    func snapshot() async -> LauncherHomeSnapshot {
        await aggregator.snapshot()
    }
}
