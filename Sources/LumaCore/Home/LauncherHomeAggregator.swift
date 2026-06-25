import Foundation

public actor LauncherHomeAggregator {
    private let openApps: any LauncherHomeProvider
    private let recent: any LauncherHomeProvider
    private let contextual: any LauncherHomeProvider

    public init(
        openApps: any LauncherHomeProvider,
        recent: any LauncherHomeProvider,
        contextual: any LauncherHomeProvider
    ) {
        self.openApps = openApps
        self.recent = recent
        self.contextual = contextual
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        async let apps = openApps.items()
        async let recents = recent.items()
        async let suggested = contextual.items()
        let sections = await [
            LauncherHomeSection(kind: .openApps, items: apps),
            LauncherHomeSection(kind: .suggested, items: suggested),
            LauncherHomeSection(kind: .recent, items: recents)
        ].filter { !$0.items.isEmpty }
        return LauncherHomeSnapshot(sections: sections)
    }
}
