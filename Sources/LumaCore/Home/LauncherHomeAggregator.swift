import Foundation

public actor LauncherHomeAggregator {
    private let openApps: any LauncherHomeProvider
    private let contextual: any LauncherHomeProvider

    public init(
        openApps: any LauncherHomeProvider,
        contextual: any LauncherHomeProvider
    ) {
        self.openApps = openApps
        self.contextual = contextual
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        async let apps = openApps.items()
        async let suggested = contextual.items()
        let sections = await [
            LauncherHomeSection(kind: .openApps, items: apps),
            LauncherHomeSection(kind: .suggested, items: suggested)
        ].filter { !$0.items.isEmpty }
        return LauncherHomeSnapshot(sections: sections)
    }
}
