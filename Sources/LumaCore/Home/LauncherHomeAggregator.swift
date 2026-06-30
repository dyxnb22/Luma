import Foundation

public actor LauncherHomeAggregator {
    private let setup: (any LauncherHomeProvider)?
    private let openApps: any LauncherHomeProvider
    private let recentActions: any LauncherHomeProvider
    private let resume: any LauncherHomeProvider
    private let contextual: any ContextualHomeSectionProvider

    public init(
        openApps: any LauncherHomeProvider,
        recentActions: any LauncherHomeProvider,
        resume: any LauncherHomeProvider,
        contextual: any ContextualHomeSectionProvider,
        setup: (any LauncherHomeProvider)? = nil
    ) {
        self.openApps = openApps
        self.recentActions = recentActions
        self.resume = resume
        self.contextual = contextual
        self.setup = setup
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        async let setupItems = setup?.items() ?? []
        async let apps = openApps.items()
        async let recent = recentActions.items()
        async let resumeItems = resume.items()
        async let contextualSections = contextual.sectionedItems()
        let loaded = await (setupItems, apps, recent, resumeItems, contextualSections)

        var continueItems = loaded.3 + loaded.4.continue
        if continueItems.count > 4 {
            continueItems = Array(continueItems.prefix(4))
        }

        var sections: [LauncherHomeSection] = []
        if !loaded.0.isEmpty {
            sections.append(LauncherHomeSection(kind: .setup, items: loaded.0))
        }
        if !loaded.1.isEmpty {
            sections.append(LauncherHomeSection(kind: .openApps, items: loaded.1))
        }
        if !loaded.2.isEmpty {
            sections.append(LauncherHomeSection(kind: .recentActions, items: loaded.2))
        }
        if !continueItems.isEmpty {
            sections.append(LauncherHomeSection(kind: .continueFlow, items: continueItems))
        }
        if !loaded.4.create.isEmpty {
            sections.append(LauncherHomeSection(kind: .create, items: loaded.4.create))
        }
        return LauncherHomeSnapshot(sections: sections)
    }
}
