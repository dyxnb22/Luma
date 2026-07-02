import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

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

        var rows: [ResultItem] = []
        if !(await config.onboardingCompleted()) {
            rows.append(onboardingRow())
        }

        let notesConfig = await notesRootStore.load()

        if await needsAccessibilitySetup() {
            rows.append(accessibilitySetupRow())
        }
        if enablementGate.contains(.notes), notesConfig.root == nil {
            rows.append(notesSetupRow())
        }
        if rows.isEmpty {
            rows.append(modulesSetupRow())
        }
        return Array(rows.prefix(HomeSuggestionPolicy.maxSetupRows))
    }

    private func onboardingRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: .commands, key: "setup.onboarding"),
            title: L10n.tr("onboarding.homeRow.title"),
            titleAttributed: AttributedString(L10n.tr("onboarding.homeRow.title")),
            subtitle: L10n.tr("onboarding.homeRow.subtitle"),
            icon: .symbol("sparkles"),
            primaryAction: Action(
                id: ActionID(module: .commands, key: "setup.onboarding"),
                title: L10n.tr("onboarding.button.next"),
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: 102),
            rowKind: .starter
        )
    }

    private func notesSetupRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: .notes, key: "setup.notes-root"),
            title: L10n.tr("setup.notes.title"),
            titleAttributed: AttributedString(L10n.tr("setup.notes.title")),
            subtitle: L10n.tr("setup.notes.subtitle"),
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
            title: L10n.tr("setup.modules.title"),
            titleAttributed: AttributedString(L10n.tr("setup.modules.title")),
            subtitle: L10n.tr("setup.modules.subtitle"),
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

    private func needsAccessibilitySetup() async -> Bool {
        guard !AXService.isProcessTrusted() else { return false }
        let defaultEnabled = Set(
            BuiltInModules.makeAll()
                .filter { type(of: $0).manifest.defaultEnabled }
                .map { type(of: $0).manifest.identifier }
        )
        let enabled = await config.enabledModules() ?? defaultEnabled
        return BuiltInModules.enabledModulesRequireAccessibility(enabled)
    }

    private func accessibilitySetupRow() -> ResultItem {
        let payload = Data("open-settings".utf8)
        return ResultItem(
            id: ResultID(module: .commands, key: "setup.accessibility"),
            title: L10n.tr("setup.accessibility.title"),
            titleAttributed: AttributedString(L10n.tr("setup.accessibility.title")),
            subtitle: L10n.tr("setup.accessibility.subtitle"),
            icon: .symbol("hand.raised"),
            primaryAction: Action(
                id: ActionID(module: .commands, key: "setup.accessibility"),
                title: L10n.tr("setup.openSettings"),
                kind: .custom(payload: payload, handler: .commands)
            ),
            rankingHints: RankingHints(basePriority: 101),
            rowKind: .starter
        )
    }
}
