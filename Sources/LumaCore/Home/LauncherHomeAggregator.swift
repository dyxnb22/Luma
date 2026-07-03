import Foundation

public actor LauncherHomeAggregator {
    private let setup: (any LauncherHomeProvider)?
    private let openApps: any LauncherHomeProvider

    public init(
        openApps: any LauncherHomeProvider,
        setup: (any LauncherHomeProvider)? = nil
    ) {
        self.openApps = openApps
        self.setup = setup
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        async let setupItems = setup?.items() ?? []
        async let apps = openApps.items()
        let loaded = await (setupItems, apps)

        var sections: [LauncherHomeSection] = []
        if !loaded.0.isEmpty {
            sections.append(LauncherHomeSection(kind: .setup, items: loaded.0))
        }
        if !loaded.1.isEmpty {
            sections.append(LauncherHomeSection(kind: .openApps, items: loaded.1))
        }
        return LauncherHomeSnapshot(sections: sections)
    }
}
