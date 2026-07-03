import Foundation

public actor LauncherHomeAggregator {
    private let openApps: any LauncherHomeProvider

    public init(openApps: any LauncherHomeProvider) {
        self.openApps = openApps
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        let apps = await openApps.items()
        guard !apps.isEmpty else { return .empty }
        return LauncherHomeSnapshot(sections: [
            LauncherHomeSection(kind: .openApps, items: apps)
        ])
    }
}
