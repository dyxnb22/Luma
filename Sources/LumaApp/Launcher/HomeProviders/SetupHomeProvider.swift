import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules

/// Lightweight first-run setup rows — not a full onboarding flow.
struct SetupHomeProvider: LauncherHomeProvider {
    private let config: ConfigurationStore
    private let enablementGate: HomeEnablementGate
    private let notesRootStore: NotesRootConfigStore

    init(
        config: ConfigurationStore,
        enablementGate: HomeEnablementGate = HomeEnablementGate(),
        notesRootStore: NotesRootConfigStore = NotesRootConfigStore()
    ) {
        self.config = config
        self.enablementGate = enablementGate
        self.notesRootStore = notesRootStore
    }

    func items() async -> [ResultItem] {
        guard await !config.setupHintsDismissed() else { return [] }

        let notesConfig = await notesRootStore.load()
        var rows: [ResultItem] = []

        if enablementGate.contains(.notes), notesConfig.root == nil {
            rows.append(notesSetupRow())
        }
        if rows.isEmpty {
            rows.append(modulesSetupRow())
        }
        return Array(rows.prefix(HomeSuggestionPolicy.maxSetupRows))
    }

    private func notesSetupRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: .notes, key: "setup.notes-root"),
            title: "Set up Notes folder",
            titleAttributed: AttributedString("Set up Notes folder"),
            subtitle: "Choose where your markdown library lives",
            icon: .symbol("folder.badge.gearshape"),
            primaryAction: Action(
                id: ActionID(module: .notes, key: "setup.notes-root"),
                title: CrossModuleActionTitles.setupNotes,
                kind: .openModuleDetail(.notes, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: 100),
            rowKind: .starter
        )
    }

    private func modulesSetupRow() -> ResultItem {
        let payload = Data("open-settings".utf8)
        return ResultItem(
            id: ResultID(module: .commands, key: "setup.modules"),
            title: "Review modules & pins",
            titleAttributed: AttributedString("Review modules & pins"),
            subtitle: "Enable what you use and pin hot-path modules",
            icon: .symbol("slider.horizontal.3"),
            primaryAction: Action(
                id: ActionID(module: .commands, key: "setup.modules"),
                title: CrossModuleActionTitles.reviewModules,
                kind: .custom(payload: payload, handler: .commands)
            ),
            rankingHints: RankingHints(basePriority: 99),
            rowKind: .starter
        )
    }
}
