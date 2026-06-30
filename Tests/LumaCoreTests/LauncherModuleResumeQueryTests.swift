import Foundation
import LumaCore
import Testing

@Test func launcherModuleResumeQueryFillsEmptyNotesQuery() {
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let query = LauncherModuleResumeQuery.normalizedQuery(for: notes, raw: "")
    #expect(query == "n ")
}

@Test func launcherModuleResumeQueryPreservesNonEmptyQuery() {
    let notes = ModuleIdentifier(rawValue: "luma.notes")
    let query = LauncherModuleResumeQuery.normalizedQuery(for: notes, raw: "n daily")
    #expect(query == "n daily")
}

@Test func launcherModuleResumeQueryTitlesAreModuleSpecific() {
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.notes")) == "Resume Notes search")
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.wordbook")) == "Resume Wordbook")
    #expect(LauncherModuleResumeQuery.resumeTitle(for: ModuleIdentifier(rawValue: "luma.media")) == "Resume Records")
}

@Test func homeAggregatorIncludesSetupSectionFirst() async {
    struct StubHomeProvider: LauncherHomeProvider {
        let rows: [ResultItem]
        func items() async -> [ResultItem] { rows }
    }

    struct StubContextualProvider: ContextualHomeSectionProvider {
        func items() async -> [ResultItem] { [] }

        func sectionedItems() async -> (continue: [ResultItem], create: [ResultItem]) {
            (continue: [], create: [])
        }
    }

    let setupID = ModuleIdentifier(rawValue: "luma.notes")
    let setupRow = ResultItem(
        id: ResultID(module: setupID, key: "setup.notes-root"),
        title: "Set up Notes",
        titleAttributed: "Set up Notes",
        icon: .symbol("folder"),
        primaryAction: Action(id: ActionID(module: setupID, key: "setup"), title: "Set up", kind: .noop),
        rankingHints: RankingHints()
    )

    let aggregator = LauncherHomeAggregator(
        openApps: StubHomeProvider(rows: []),
        recentActions: StubHomeProvider(rows: []),
        resume: StubHomeProvider(rows: []),
        contextual: StubContextualProvider(),
        setup: StubHomeProvider(rows: [setupRow])
    )
    let snapshot = await aggregator.snapshot()
    #expect(snapshot.sections.first?.kind == .setup)
    #expect(snapshot.sections.first?.items.first?.title == "Set up Notes")
}
