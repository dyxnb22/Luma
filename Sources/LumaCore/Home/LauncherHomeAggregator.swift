import Foundation

public actor LauncherHomeAggregator {
    private let openApps: any LauncherHomeProvider
    private let recentActions: any LauncherHomeProvider
    private let resume: any LauncherHomeProvider
    private let contextual: any ContextualHomeSectionProvider

    public init(
        openApps: any LauncherHomeProvider,
        recentActions: any LauncherHomeProvider,
        resume: any LauncherHomeProvider,
        contextual: any ContextualHomeSectionProvider
    ) {
        self.openApps = openApps
        self.recentActions = recentActions
        self.resume = resume
        self.contextual = contextual
    }

    public func snapshot() async -> LauncherHomeSnapshot {
        async let apps = openApps.items()
        async let recent = recentActions.items()
        async let resumeItems = resume.items()
        async let contextualSections = contextual.sectionedItems()
        let loaded = await (apps, recent, resumeItems, contextualSections)

        var continueItems = loaded.2 + loaded.3.continue
        if continueItems.count > 4 {
            continueItems = Array(continueItems.prefix(4))
        }

        var sections: [LauncherHomeSection] = []
        if !loaded.0.isEmpty {
            sections.append(LauncherHomeSection(kind: .openApps, items: loaded.0))
        }
        if !loaded.1.isEmpty {
            sections.append(LauncherHomeSection(kind: .recentActions, items: loaded.1))
        }
        if !continueItems.isEmpty {
            sections.append(LauncherHomeSection(kind: .continueFlow, items: continueItems))
        }
        if !loaded.3.create.isEmpty {
            sections.append(LauncherHomeSection(kind: .create, items: loaded.3.create))
        }
        return LauncherHomeSnapshot(sections: sections)
    }
}
