import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

/// Presents module detail in-panel (warmup, cross-fade).
/// Search-field detail mode is entered via `enterDetailContext`; exited via chrome `exitDetailFromChrome`.
@MainActor
protocol LauncherDetailPresenting: AnyObject {
    var isPanelActiveForQueryApply: Bool { get }
    var lastRenderedHomeGeneration: UInt64 { get set }
    var lastSplitLayoutState: (columnSplit: Bool, rightPane: LauncherSplitRightPane)? { get set }
    func applySessionEvent(_ event: LauncherSessionEvent)
    func showStatus(_ message: String)
    func showHome(focusSearch: Bool, persist: Bool)
    func usesColumnSplitLayout() -> Bool
    func syncSplitLayout()
    func syncRowActionHint()
    func syncPerformanceStripVisibility()
    func stabilizePanelContentLayout()
    func refreshPermissionBannerCoalesced()
}

@MainActor
final class LauncherDetailPresenter {
    private weak var host: LauncherDetailPresenting?
    private let config: ConfigurationStore
    private let sessionStore: LauncherSessionStore
    private let taskRegistry: LauncherTaskRegistry
    private let detailLifecycle: LauncherDetailLifecycleController
    private let launcherEnvironment: LauncherEnvironment
    private let homeCoordinator: LauncherHomeCoordinator
    private let contentCoordinator: LauncherContentCoordinator
    private let searchBar: LumaSearchBar
    private let hintBar: LauncherHintBar
    private let homeSplitLayout: LauncherHomeSplitLayout

    init(
        host: LauncherDetailPresenting,
        config: ConfigurationStore,
        sessionStore: LauncherSessionStore,
        taskRegistry: LauncherTaskRegistry,
        detailLifecycle: LauncherDetailLifecycleController,
        launcherEnvironment: LauncherEnvironment,
        homeCoordinator: LauncherHomeCoordinator,
        contentCoordinator: LauncherContentCoordinator,
        searchBar: LumaSearchBar,
        hintBar: LauncherHintBar,
        homeSplitLayout: LauncherHomeSplitLayout
    ) {
        self.host = host
        self.config = config
        self.sessionStore = sessionStore
        self.taskRegistry = taskRegistry
        self.detailLifecycle = detailLifecycle
        self.launcherEnvironment = launcherEnvironment
        self.homeCoordinator = homeCoordinator
        self.contentCoordinator = contentCoordinator
        self.searchBar = searchBar
        self.hintBar = hintBar
        self.homeSplitLayout = homeSplitLayout
    }

    func openModuleDetail(for moduleID: ModuleIdentifier, payload: Data? = nil) {
        Task { @MainActor in
            guard await isModuleEnabledForDetail(moduleID) else {
                host?.showStatus(LauncherStatusMessages.moduleDisabledInSettings)
                return
            }
            if moduleID == .snippets, let payload,
               let action = try? ModuleActionCoding.decode(SnippetsAction.self, from: payload) {
                switch action {
                case .prepareDraft(let draft):
                    LauncherSharedState.pendingSnippetDraft = draft
                case .create(let title):
                    LauncherSharedState.pendingSnippetDraft = SnippetDraft(title: title, content: "")
                default:
                    break
                }
                if let message = draftLoadedStatus(for: moduleID, payload: payload) {
                    launcherEnvironment.showStatus(message)
                }
                presentModuleDetail(for: moduleID)
                return
            }
            applyModuleDetailPayload(moduleID: moduleID, payload: payload)
            if let message = draftLoadedStatus(for: moduleID, payload: payload) {
                launcherEnvironment.showStatus(message)
            }
            presentModuleDetail(for: moduleID)
        }
    }

    private func draftLoadedStatus(for moduleID: ModuleIdentifier, payload: Data?) -> String? {
        guard let payload else { return nil }
        let probe = Action(
            id: ActionID(module: moduleID, key: "draft-probe"),
            title: "",
            kind: .openModuleDetail(moduleID, payload: payload)
        )
        return LauncherActionFeedback.statusMessage(for: probe)
    }

    private func isModuleEnabledForDetail(_ moduleID: ModuleIdentifier) async -> Bool {
        guard let enabled = await config.enabledModules() else {
            return ModuleRegistry.defaultEnabledModuleIDs.contains(moduleID)
        }
        return enabled.contains(moduleID)
    }

    private func presentModuleDetail(for moduleID: ModuleIdentifier) {
        let generation = detailLifecycle.nextPresentationGeneration()
        let task = Task { @MainActor [weak self] in
            guard let self, let host = self.host else { return }
            sessionStore.flushPendingWrites()
            await launcherEnvironment.warmModuleForDetail(moduleID)
            guard detailLifecycle.isPresentationGenerationCurrent(generation) else { return }
            guard await isModuleEnabledForDetail(moduleID) else {
                host.showStatus(LauncherStatusMessages.moduleDisabledInSettings)
                return
            }
            guard host.isPanelActiveForQueryApply || contentCoordinator.showingDetail else { return }
            guard let detail = launcherEnvironment.makeDetailView(for: moduleID) else {
                host.showHome(focusSearch: true, persist: false)
                return
            }
            host.applySessionEvent(.detailOpened(moduleID, suspendedQuery: searchBar.persistedQuery))
            let presentation = moduleDetailPresentation()
            if let cached = await homeCoordinator.cachedSnapshotIfAvailable() {
                contentCoordinator.showHome(cached, preserveSelection: true)
                host.lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
            } else {
                let snapshot = await homeCoordinator.snapshot()
                contentCoordinator.showHome(snapshot, preserveSelection: true)
                host.lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
            }
            enterDetailContext(moduleTitle: detail.moduleTitle)

            let finishPresentation = { [weak self] in
                guard let self, let host = self.host else { return }
                guard detailLifecycle.isPresentationGenerationCurrent(generation) else { return }
                if moduleID == .translate,
                   let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
                    let state = LauncherResumeStore.load()
                    if !state.translateSource.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        translate.restore(sourceText: state.translateSource, outputText: state.translateOutput)
                    }
                }
                host.lastSplitLayoutState = (host.usesColumnSplitLayout(), .detail)
                host.syncSplitLayout()
                host.syncRowActionHint()
                host.syncPerformanceStripVisibility()
                host.stabilizePanelContentLayout()
                host.refreshPermissionBannerCoalesced()
            }

            if host.usesColumnSplitLayout() {
                homeSplitLayout.crossfadeFromGuideToDetail { [weak self] in
                    guard let self else { return }
                    contentCoordinator.present(
                        detail,
                        moduleID: moduleID,
                        presentation: presentation,
                        stagedForCrossfade: true
                    )
                    Task { @MainActor in
                        await self.launcherEnvironment.activateDetail(detail, for: moduleID)
                    }
                } completion: {
                    finishPresentation()
                    LauncherPerfCounters.increment(.detailOpen)
                }
            } else {
                contentCoordinator.present(detail, moduleID: moduleID, presentation: presentation)
                await launcherEnvironment.activateDetail(detail, for: moduleID)
                finishPresentation()
                LauncherPerfCounters.increment(.detailOpen)
            }
        }
        taskRegistry.register(key: "presentModuleDetail", task: task)
    }

    func applyModuleDetailPayload(moduleID: ModuleIdentifier, payload: Data?) {
        guard let payload else { return }
        if moduleID == .media,
           let action = try? ModuleActionCoding.decode(MediaAction.self, from: payload) {
            switch action {
            case .editDraft(let draft):
                LauncherSharedState.pendingMediaEditorDraft = draft
            case .openDetail:
                LauncherSharedState.pendingMediaEditorDraft = nil
            default:
                break
            }
            return
        }
        if moduleID == .wordbook,
           let action = try? ModuleActionCoding.decode(WordbookAction.self, from: payload),
           action == .review {
            LauncherSharedState.pendingWordbookAutoStartReview = true
        }
        if moduleID == .projects,
           let action = try? ModuleActionCoding.decode(ProjectAction.self, from: payload) {
            switch action {
            case .openCurrentDetail(let context):
                LauncherSharedState.pendingCurrentProjectContext = context
            case .openManage:
                LauncherSharedState.pendingProjectsManage = true
            default:
                break
            }
            return
        }
        if moduleID == .quicklinks,
           let action = try? ModuleActionCoding.decode(QuicklinksAction.self, from: payload),
           case .prepareDraft(let draft) = action {
            LauncherSharedState.pendingQuicklinkDraft = draft
        }
    }

    private func enterDetailContext(moduleTitle: String) {
        searchBar.beginDetailMode(moduleTitle: moduleTitle)
        hintBar.setContext(.detail)
    }

    private func moduleDetailPresentation() -> ModuleDetailPresentation {
        .rightColumn
    }
}
