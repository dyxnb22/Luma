import Foundation

public actor LauncherHomeAggregator {
    private let openApps: any LauncherHomeProvider

    public init(openApps: any LauncherHomeProvider) {
        self.openApps = openApps
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        let apps = await openApps.items()
        let warming = await openApps.isWarming()
        return LauncherHomeSnapshot(sections: [
            LauncherHomeSection(kind: .openApps, items: apps, isWarming: warming && apps.isEmpty)
        ])
    }
}
