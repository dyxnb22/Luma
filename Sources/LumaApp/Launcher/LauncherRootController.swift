import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class LauncherRootController {
    let searchBar: LumaSearchBar
    let commandHintBar: CommandHintBar
    let listView: LauncherListView
    let hintBar: LauncherHintBar
    let actionPanel: LauncherActionPanel
    let contentCoordinator: LauncherContentCoordinator
    let permissionController: PermissionBannerController

    private let homeSplitLayout: LauncherHomeSplitLayout
    private let performanceStrip: LauncherPerformanceStripView

    private let viewModel: LauncherViewModel
    private let homeCoordinator: LauncherHomeCoordinator
    private let actionExecutor: ActionExecutor
    private let config: ConfigurationStore
    private let sessionStore = LauncherSessionStore()
    private let panelSignalsLoader: LauncherPanelSignalsLoader
    private let launcherEnvironment: LauncherEnvironment
    private let workbenchCommandExecutor = WorkbenchCommandExecutor()
    private let onDismiss: () -> Void
    private let onActionDismiss: () -> Void
    private let onOpenSettings: () -> Void

    private var modulesReady = false
    private var homeRefreshTask: Task<Void, Never>?
    private var querySyncTimer: Timer?
    private var lastSyncedQuery = ""
    private var lastSplitLayoutState: (columnSplit: Bool, rightPane: LauncherSplitRightPane)?
    private var lastResultsRouteKind: NormalizedQueryState.ResultsRouteKind?
    private var lastNormalizedQueryState: NormalizedQueryState?
    private var permissionRefreshTask: Task<Void, Never>?
    private var lastAppliedGuideCatalogIDs: [String] = []
    private lazy var snapshotApplyCoalescer = LauncherSnapshotApplyCoalescer { [weak self] snapshot in
        self?.apply(snapshot: snapshot)
    }
    private var restoreGeneration: UInt = 0
    private var querySyncGraceUntil: ContinuousClock.Instant?

    init(
        viewModel: LauncherViewModel,
        homeCoordinator: LauncherHomeCoordinator,
        actionExecutor: ActionExecutor,
        config: ConfigurationStore,
        contentCoordinator: LauncherContentCoordinator,
        searchBar: LumaSearchBar,
        commandHintBar: CommandHintBar,
        listView: LauncherListView,
        hintBar: LauncherHintBar,
        actionPanel: LauncherActionPanel,
        homeSplitLayout: LauncherHomeSplitLayout,
        performanceStrip: LauncherPerformanceStripView,
        launcherEnvironment: LauncherEnvironment,
        onDismiss: @escaping () -> Void,
        onActionDismiss: @escaping () -> Void,
        onOpenSettings: @escaping () -> Void
    ) {
        self.viewModel = viewModel
        self.homeCoordinator = homeCoordinator
        self.actionExecutor = actionExecutor
        self.config = config
        self.contentCoordinator = contentCoordinator
        self.searchBar = searchBar
        self.commandHintBar = commandHintBar
        self.listView = listView
        self.hintBar = hintBar
        self.actionPanel = actionPanel
        self.homeSplitLayout = homeSplitLayout
        self.performanceStrip = performanceStrip
        self.launcherEnvironment = launcherEnvironment
        self.onDismiss = onDismiss
        self.onActionDismiss = onActionDismiss
        self.onOpenSettings = onOpenSettings
        self.panelSignalsLoader = LauncherPanelSignalsLoader(config: config)
        self.permissionController = PermissionBannerController(config: config)

        wireCallbacks()
        permissionController.onOpenSettings = { [weak self] in self?.onOpenSettings() }
    }

    private func wireCallbacks() {
        contentCoordinator.onSessionChanged = { [weak self] in self?.saveCurrentSession() }
        contentCoordinator.onHomeSessionSaved = { [weak self] in self?.saveHomeSession() }
        contentCoordinator.onRun = { [weak self] item in self?.handleRun(item) }
        contentCoordinator.onRightClick = { [weak self] item in self?.openActionPanel(for: item) }
        contentCoordinator.onSelectionChanged = { [weak self] in self?.syncRowActionHint() }

        listView.onKeyCommand = { [weak self] command in self?.handleKeyCommand(command) ?? false }
        listView.onInterceptKeyDown = { [weak self] event in
            self?.actionPanel.handleKeyDown(event) ?? false
        }
        listView.onActivate = { [weak self] in self?.activateReturn() }
        listView.onEscape = { [weak self] in self?.handleEscape() }
        listView.onTypeToSearch = { [weak self] text in self?.forwardTypingToSearch(text) }

        actionPanel.onRun = { [weak self] action, item in self?.runAction(action, for: item) }
        actionPanel.onClose = { [weak self] in self?.focusSearchField() }

        searchBar.onEscape = { [weak self] in self?.handleEscape() }
        searchBar.onReturn = { [weak self] in self?.activateReturn() }
        searchBar.onInterceptKeyDown = { [weak self] event in
            self?.actionPanel.handleKeyDown(event) ?? false
        }
        searchBar.onTextChange = { [weak self] text in
            self?.noteSearchTextChangedForQuerySync()
            self?.handleTextChange(text)
        }
        searchBar.onKeyCommand = { [weak self] command in self?.handleKeyCommand(command) ?? false }
        searchBar.onOpenSettings = onOpenSettings
        searchBar.onDetailKey = { [weak self] event in
            guard let self, self.contentCoordinator.showingDetail else { return false }
            if LumaStandardEditShortcuts.handleKeyDown(event, in: self.searchBar.window) {
                return true
            }
            guard self.searchBar.stringValue.isEmpty else { return false }
            return self.contentCoordinator.currentDetailObject?.handleKeyDown(event) ?? false
        }

        viewModel.onSnapshot = { [weak self] snapshot in self?.enqueueSnapshotApply(snapshot) }
    }

    private func previewWorkbenchCommand(route: WorkbenchCommandRoute, queryText: String) async {
        let needsSelection: Bool
        switch route {
        case .attachSelection, .capture, .projectCapture:
            needsSelection = true
        default:
            needsSelection = false
        }
        let context = await panelSignalsLoader.loadWorkbenchContext(includeSelection: needsSelection)
        let items = WorkbenchCommandResults.previewRows(
            route: route,
            querySequence: 0,
            context: context
        )
        await MainActor.run {
            contentCoordinator.apply(snapshot: ResultSnapshot(querySequence: 0, items: items))
            syncRowActionHint()
        }
    }

    @discardableResult
    private func applyWorkbenchCommandOutcome(
        _ outcome: WorkbenchCommandOutcome,
        replaceQueryHandler: ((String) -> Void)? = nil
    ) -> Bool {
        switch outcome {
        case .notHandled:
            return false
        case .status(let message):
            showStatus(message)
            contentCoordinator.beginShowingResults()
            listView.isHidden = false
            contentCoordinator.apply(snapshot: ResultSnapshot(querySequence: 0, items: []))
            return true
        case .openDetail(let moduleID, let payload):
            openModuleDetail(for: moduleID, payload: payload)
            return true
        case .replaceQuery(let text):
            if let replaceQueryHandler {
                replaceQueryHandler(text)
            } else {
                searchBar.stringValue = text
                lastSyncedQuery = ""
                handleTextChange(text)
            }
            return true
        case .runAction(let kind):
            let action = Action(
                id: ActionID(module: .notes, key: "workbench.capture"),
                title: "Capture",
                kind: kind
            )
            Task {
                await actionExecutor.run(action, for: ResultID(module: .notes, key: "workbench.capture"))
                await MainActor.run { self.onActionDismiss() }
            }
            return true
        case .resumeActivity(let entryID):
            runWorkbenchResumeActivity(entryID: entryID, for: ResultItem(
                id: ResultID(module: .workbench, key: "command.resume.\(entryID.uuidString)"),
                title: "Resume activity",
                titleAttributed: AttributedString("Resume activity"),
                subtitle: nil,
                icon: .symbol("clock.arrow.circlepath"),
                primaryAction: Action(
                    id: ActionID(module: .workbench, key: "command.resume"),
                    title: "Resume",
                    kind: .noop
                ),
                rankingHints: RankingHints(basePriority: 0),
            ))
            return true
        case .openLinked(let linkID):
            runOpenLinkedEntity(linkID: linkID)
            return true
        case .openActivityEntry(let entryID):
            runOpenActivityEntry(entryID: entryID, for: nil)
            return true
        }
    }

    func setModulesReady(_ ready: Bool) {
        modulesReady = ready
        hintBar.setModulesReady(ready)
        if ready, searchBar.stringValue.isEmpty {
            refreshHome()
        } else if ready {
            handleTextChange(searchBar.queryText)
        }
    }

    func startQuerySync() {
        stopQuerySync()
        lastSyncedQuery = searchBar.queryText
        querySyncGraceUntil = ContinuousClock.now.advanced(by: .seconds(1))
        ensureQuerySyncTimerIfNeeded()
    }

    func stopQuerySync() {
        querySyncTimer?.invalidate()
        querySyncTimer = nil
        querySyncGraceUntil = nil
    }

    func noteSearchTextChangedForQuerySync() {
        if searchBar.isComposing {
            ensureQuerySyncTimerIfNeeded()
        }
    }

    private func ensureQuerySyncTimerIfNeeded() {
        guard querySyncTimer == nil else { return }
        guard needsQuerySyncPolling else { return }
        querySyncTimer = Timer.scheduledTimer(withTimeInterval: 0.2, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.syncQueryIfNeeded()
            }
        }
    }

    private var needsQuerySyncPolling: Bool {
        if searchBar.isComposing { return true }
        if let grace = querySyncGraceUntil, ContinuousClock.now < grace { return true }
        return false
    }

    private func syncQueryIfNeeded() {
        if !needsQuerySyncPolling {
            stopQuerySync()
            return
        }
        let text = searchBar.queryText
        guard text != lastSyncedQuery else { return }
        handleTextChange(text)
    }

    func showHome(focusSearch: Bool = true, persist: Bool = true) {
        homeRefreshTask?.cancel()
        viewModel.cancel()
        cancelPendingSnapshotApply()
        searchBar.cancelDetailMode()
        searchBar.resetQueryText()
        searchBar.setPlaceholder(ModuleSearchHints.cheatSheet)
        lastSyncedQuery = ""
        commandHintBar.apply(nil)
        launcherEnvironment.isLauncherQueryEmpty = true
        contentCoordinator.tearDownDetailIfNeeded()
        contentCoordinator.resetResults()
        listView.clear()
        listView.isHidden = false
        listView.alphaValue = 1
        syncKeyHints()
        syncSplitLayout()
        syncPerformanceStripVisibility()
        refreshHome()
        refreshPermissionBanner()
        if focusSearch { focusSearchField() }
        if persist {
            saveHomeSession()
            persistResumeState(translateContent: nil)
        }
    }

    private var lastRenderedHomeGeneration: UInt64 = 0

    func refreshHome(preserveListSelection: Bool = false, force: Bool = false) {
        homeRefreshTask?.cancel()
        homeRefreshTask = Task {
            if !force {
                let generation = await homeCoordinator.currentSnapshotGeneration()
                if generation == self.lastRenderedHomeGeneration,
                   await homeCoordinator.cachedSnapshotIfAvailable() != nil {
                    return
                }
            }
            let snapshot = await homeCoordinator.snapshot()
            guard !Task.isCancelled else { return }
            await MainActor.run {
                let trimmed = self.searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
                guard trimmed.isEmpty else { return }
                if self.contentCoordinator.showingDetail {
                    self.contentCoordinator.showHome(snapshot, preserveSelection: true)
                    self.syncRowActionHint()
                    return
                }
                guard !self.contentCoordinator.showingResults,
                      self.launcherEnvironment.isLauncherQueryEmpty else { return }
                self.contentCoordinator.showHome(
                    snapshot,
                    preserveSelection: preserveListSelection
                )
                self.syncSplitLayout()
                self.syncRowActionHint()
                _ = HomeLatencyTracker.markHomeRendered()
            }
            self.lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
        }
    }

    /// Restores empty-query home after detail exit without clearing the left Open Apps list.
    private func restoreHomeFromDetail(persist: Bool) {
        homeRefreshTask?.cancel()
        viewModel.cancel()
        cancelPendingSnapshotApply()
        searchBar.cancelDetailMode()
        searchBar.resetQueryText()
        searchBar.setPlaceholder(ModuleSearchHints.cheatSheet)
        lastSyncedQuery = ""
        commandHintBar.apply(nil)
        launcherEnvironment.isLauncherQueryEmpty = true
        listView.isHidden = false
        listView.alphaValue = 1
        syncKeyHints()
        syncPerformanceStripVisibility()
        Task { @MainActor in
            if let cached = await homeCoordinator.cachedSnapshotIfAvailable() {
                contentCoordinator.showHome(cached, preserveSelection: true)
                syncSplitLayout()
                syncRowActionHint()
                lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
                LauncherPerfCounters.increment(.backHome)
            } else {
                refreshHome(preserveListSelection: true, force: true)
            }
            refreshPermissionBannerCoalesced()
            if persist {
                saveHomeSession()
                persistResumeState(translateContent: nil)
            }
        }
    }

    private func syncSplitLayoutIfSearchLayoutMayHaveChanged() {
        let columnSplit = usesColumnSplitLayout()
        if !columnSplit, lastSplitLayoutState?.columnSplit == false {
            return
        }
        syncSplitLayout()
    }

    private func shouldClearStaleResults(for kind: NormalizedQueryState.ResultsRouteKind) -> Bool {
        guard let lastResultsRouteKind else { return false }
        return kind != lastResultsRouteKind
    }

    func resetHomeExpansion() {
        Task {
            await homeCoordinator.resetExpansion()
        }
    }

    func toggleOpenAppWindows(bundleID: String) {
        Task {
            await homeCoordinator.toggleAppWindows(bundleID: bundleID)
            await MainActor.run {
                self.refreshHome()
                self.refreshPermissionBanner()
            }
        }
    }

    func focusSearchField() {
        if contentCoordinator.showingDetail {
            contentCoordinator.currentDetailObject?.activate()
            return
        }
        searchBar.reEnableSearchFieldIfNeeded()
        searchBar.focus()
    }

    func focusSearchFieldAfterShow() {
        if contentCoordinator.showingDetail {
            contentCoordinator.currentDetailObject?.activate()
            return
        }
        if !searchBar.stringValue.isEmpty {
            searchBar.focus()
            return
        }
        searchBar.reEnableSearchFieldIfNeeded()
        searchBar.focus()
    }

    private func forwardTypingToSearch(_ text: String) {
        guard !contentCoordinator.showingDetail, !actionPanel.isVisible else { return }
        searchBar.focus()
        searchBar.appendText(text)
        lastSyncedQuery = ""
        handleTextChange(searchBar.stringValue)
    }

    func cancelPendingRestore() {
        restoreGeneration &+= 1
        snapshotApplyCoalescer.cancel()
    }

    func restoreLastSessionIfNeeded() {
        guard ProcessInfo.processInfo.environment["LUMA_QA"] != "1" else { return }
        if contentCoordinator.showingDetail {
            focusSearchField()
            return
        }
        guard searchBar.stringValue.isEmpty else { return }
        let generation = restoreGeneration
        Task {
            let persisted = await sessionStore.loadPersistedSession()
            await MainActor.run {
                guard self.restoreGeneration == generation else { return }
                self.applyRestore(
                    moduleRaw: persisted.moduleRaw,
                    query: persisted.query,
                    translateSource: persisted.translateSource,
                    translateOutput: persisted.translateOutput
                )
                self.focusSearchField()
            }
        }
    }

    func dispatchDetailKeyDown(_ event: NSEvent) -> Bool {
        guard contentCoordinator.showingDetail else { return false }
        if LumaStandardEditShortcuts.handleKeyDown(event, in: searchBar.window) {
            return true
        }
        return contentCoordinator.currentDetailObject?.handleKeyDown(event) ?? false
    }

    @discardableResult
    func dispatchDetailCloseFromKeyboard() -> Bool {
        guard contentCoordinator.showingDetail else { return false }
        exitDetailFromChrome()
        return true
    }

    var isShowingDetail: Bool { contentCoordinator.showingDetail }

    func saveCurrentSession() {
        let translateContent = (contentCoordinator.currentDetailObject as? TranslateDetailView)?.currentContent()
        sessionStore.saveCurrentSession(
            moduleID: contentCoordinator.currentDetailModuleID,
            query: searchBar.persistedQuery,
            translateContent: translateContent.map { ($0.source, $0.output) }
        )
        persistResumeState(translateContent: translateContent)
    }

    private func persistResumeState(translateContent: (source: String, output: String)?) {
        var query = searchBar.persistedQuery
        if let moduleID = contentCoordinator.currentDetailModuleID,
           LauncherModuleResumeQuery.roundTripModules.contains(moduleID) {
            query = LauncherModuleResumeQuery.normalizedQuery(for: moduleID, raw: query)
        }

        var state = LauncherResumeState(
            moduleRaw: contentCoordinator.currentDetailModuleID?.rawValue,
            query: query
        )

        let resolvedTranslate = translateContent
            ?? (contentCoordinator.currentDetailObject as? TranslateDetailView)?.currentContent()
        if let resolvedTranslate {
            let source = resolvedTranslate.source.trimmingCharacters(in: .whitespacesAndNewlines)
            if !source.isEmpty {
                state.translateSource = resolvedTranslate.source
                state.translateOutput = resolvedTranslate.output
            }
        }

        if let draft = LauncherSharedState.pendingSnippetDraft,
           let data = try? JSONEncoder().encode(draft) {
            state.snippetDraftJSON = data
        }

        if let draft = LauncherSharedState.pendingQuicklinkDraft,
           let data = try? JSONEncoder().encode(draft) {
            state.quicklinkDraftJSON = data
        }

        if let todoText = (contentCoordinator.currentDetailObject as? TodoDetailView)?.pendingCaptureText() {
            state.todoCaptureText = todoText
        } else {
            let trimmedQuery = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
            if let payload = TodoModule.extractPayload(raw: trimmedQuery), !payload.isEmpty {
                state.todoCaptureText = payload
            }
        }

        sessionStore.scheduleResumeSave(state)
    }

    func invalidatePanelSignalsCache() {
        Task { await panelSignalsLoader.invalidateCache() }
    }

    func invalidatePermissionModuleCache() {
        permissionController.invalidateEnabledModuleCache()
    }

    func flushPendingSessionWrites() {
        sessionStore.flushPendingWrites()
    }

    func saveHomeSession() {
        sessionStore.saveHomeSession(query: searchBar.stringValue)
    }

    func resetForActionDismiss() {
        if contentCoordinator.showingDetail {
            saveCurrentSession()
            return
        }
        sessionStore.suppressPersistence = true
        searchBar.stringValue = ""
        launcherEnvironment.isLauncherQueryEmpty = true
        contentCoordinator.resetResults()
        viewModel.cancel()
        sessionStore.suppressPersistence = false
        saveHomeSession()
        persistResumeState(translateContent: nil)
        refreshHome()
    }

    func openModuleDetail(for moduleID: ModuleIdentifier, payload: Data? = nil) {
        Task { @MainActor in
            guard await isModuleEnabledForDetail(moduleID) else {
                showStatus(LauncherStatusMessages.moduleDisabledInSettings)
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
        guard let enabled = await config.enabledModules() else { return true }
        return enabled.contains(moduleID)
    }

    private func presentModuleDetail(for moduleID: ModuleIdentifier) {
        Task { @MainActor in
            sessionStore.flushPendingWrites()
            await launcherEnvironment.warmModuleForDetail(moduleID)
            guard let detail = launcherEnvironment.makeDetailView(for: moduleID) else {
                showHome(focusSearch: true, persist: false)
                return
            }
            let presentation = moduleDetailPresentation()
            if let cached = await homeCoordinator.cachedSnapshotIfAvailable() {
                contentCoordinator.showHome(cached, preserveSelection: true)
                lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
            } else {
                let snapshot = await homeCoordinator.snapshot()
                contentCoordinator.showHome(snapshot, preserveSelection: true)
                lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
            }
            enterDetailContext(moduleTitle: detail.moduleTitle)

            let finishPresentation = { [weak self] in
                guard let self else { return }
                if moduleID == .translate,
                   let translate = self.contentCoordinator.currentDetailObject as? TranslateDetailView {
                    let state = LauncherResumeStore.load()
                    if !state.translateSource.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        translate.restore(sourceText: state.translateSource, outputText: state.translateOutput)
                    }
                }
                self.lastSplitLayoutState = (self.usesColumnSplitLayout(), .detail)
                self.syncSplitLayout()
                self.syncRowActionHint()
                self.syncPerformanceStripVisibility()
                self.stabilizePanelContentLayout()
                self.refreshPermissionBannerCoalesced()
            }

            if usesColumnSplitLayout() {
                homeSplitLayout.crossfadeFromGuideToDetail { [weak self] in
                    guard let self else { return }
                    self.contentCoordinator.present(
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
    }

    private func applyModuleDetailPayload(moduleID: ModuleIdentifier, payload: Data?) {
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

    func handleEscape() {
        let trimmed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        switch LauncherEscapePlanner.nextStep(
            actionPanelVisible: actionPanel.isVisible,
            showingDetail: contentCoordinator.showingDetail,
            showingResults: contentCoordinator.showingResults,
            queryTrimmedIsEmpty: trimmed.isEmpty
        ) {
        case .dismissActionPanel:
            actionPanel.dismiss()
        case .detailEscapeOrExit:
            if let escapeEvent = NSEvent.keyEvent(
                with: .keyDown,
                location: .zero,
                modifierFlags: [],
                timestamp: 0,
                windowNumber: Int(searchBar.window?.windowNumber ?? 0),
                context: nil,
                characters: "",
                charactersIgnoringModifiers: "",
                isARepeat: false,
                keyCode: 53
            ), dispatchDetailKeyDown(escapeEvent) {
                return
            }
            exitDetailFromChrome()
        case .showHome:
            showHome(focusSearch: true, persist: true)
        case .dismissPanel:
            flushPendingSessionWrites()
            onDismiss()
        }
    }

    func exitDetailFromChrome() {
        guard contentCoordinator.showingDetail else {
            searchBar.reEnableSearchFieldIfNeeded()
            focusSearchField()
            return
        }
        let restored = searchBar.endDetailMode()
        if let restored,
           !restored.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            closeDetail(animatedToGuide: false)
            searchBar.stringValue = restored
            lastSyncedQuery = ""
            handleTextChange(restored)
            focusSearchField()
            return
        }
        closeDetail(animatedToGuide: usesColumnSplitLayout()) { [weak self] in
            guard let self else { return }
            self.restoreHomeFromDetail(persist: true)
            self.focusSearchField()
        }
    }

    func closeDetail(animatedToGuide: Bool = false, completion: (@MainActor () -> Void)? = nil) {
        guard contentCoordinator.showingDetail else {
            completion?()
            return
        }
        Task { await launcherEnvironment.reserveDetailModule(nil) }

        if animatedToGuide, usesColumnSplitLayout() {
            let commands = viewModel.commandRouter.registry.discoverableCommands
            lastAppliedGuideCatalogIDs = commands.map(\.id)
            homeSplitLayout.crossfadeFromDetailToGuide(commands: commands) { [weak self] in
                self?.tearDownDetailAfterGuideCrossfade()
                completion?()
            }
        } else {
            syncSplitLayout()
            tearDownDetailAfterGuideCrossfade()
            completion?()
        }
    }

    private func tearDownDetailAfterGuideCrossfade() {
        contentCoordinator.closeDetail(presentation: .rightColumn)
        lastSplitLayoutState = (usesColumnSplitLayout(), .guide)
        syncKeyHints()
        syncPerformanceStripVisibility()
        searchBar.clearStuckDetailModeState()
        refreshPermissionBanner()
    }

    func showStatus(_ message: String) {
        commandHintBar.showStatus(message)
    }

    func handleTextChange(_ text: String) {
        guard text != lastSyncedQuery else { return }
        lastSyncedQuery = text
        let queryState = NormalizedQueryState(raw: text, viewModel: viewModel)
        lastNormalizedQueryState = queryState
        launcherEnvironment.isLauncherQueryEmpty = queryState.trimmed.isEmpty
        searchBar.setPlaceholder(ModuleSearchHints.placeholder(for: text))
        commandHintBar.apply(queryState.hint, helpTrigger: queryState.helpTrigger)
        LauncherPerfCounters.increment(.layoutHint)
        sessionStore.saveSearchQuery(text)
        if queryState.trimmed.isEmpty {
            lastResultsRouteKind = .empty
            searchBar.cancelDetailMode()
            contentCoordinator.dismissResultsForEmptyQuery()
            viewModel.cancel()
            cancelPendingSnapshotApply()
            syncSplitLayout()
            syncKeyHints()
            refreshHome()
            refreshPermissionBannerCoalesced()
            return
        }
        syncSplitLayoutIfSearchLayoutMayHaveChanged()
        guard modulesReady else { return }
        homeRefreshTask?.cancel()
        if contentCoordinator.showingDetail {
            searchBar.cancelDetailMode()
            closeDetail()
        }
        listView.isHidden = false
        listView.alphaValue = 1
        let clearStale = shouldClearStaleResults(for: queryState.resultsRouteKind)
        contentCoordinator.beginShowingResults(clearStaleContent: clearStale)
        lastResultsRouteKind = queryState.resultsRouteKind
        syncKeyHints()
        syncPerformanceStripVisibility()
        if queryState.workbenchRoute != .none {
            Task {
                await self.previewWorkbenchCommand(route: queryState.workbenchRoute, queryText: text)
            }
            return
        }
        let route = queryState.commandRoute
        if case .globalSearch = route, queryState.trimmed.count < 2 {
            listView.clear()
            contentCoordinator.beginShowingResults(clearStaleContent: true)
            syncKeyHints()
            syncPerformanceStripVisibility()
            let message = SearchEmptyState.message(
                for: route,
                query: text,
                registry: viewModel.commandRouter.registry
            )
            commandHintBar.showStatus(message)
            viewModel.cancel()
            cancelPendingSnapshotApply()
            refreshPermissionBannerCoalesced()
            return
        }
        let parsed = viewModel.commandRouter.registry.parsedCommand(for: text, route: route)
        viewModel.queryChanged(text, issuedAt: .now, route: route, parsedCommand: parsed)
    }

    func refreshPermissionBanner() {
        permissionRefreshTask?.cancel()
        permissionRefreshTask = Task { @MainActor [weak self] in
            await self?.refreshPermissionBannerNow()
        }
    }

    private func refreshPermissionBannerCoalesced() {
        permissionRefreshTask?.cancel()
        permissionRefreshTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(16))
            guard !Task.isCancelled else { return }
            await self?.refreshPermissionBannerNow()
        }
    }

    private func refreshPermissionBannerNow() async {
        let context = await currentAccessibilityGuidanceContext()
        permissionController.refresh(context: context)
    }

    private func currentAccessibilityGuidanceContext() async -> AccessibilityGuidanceContext {
        if contentCoordinator.showingDetail,
           let moduleID = contentCoordinator.currentDetailModuleID,
           AccessibilityGuidancePolicy.isGuidanceModule(moduleID) {
            return AccessibilityGuidanceContext(surface: .moduleDetail(moduleID))
        }

        let trimmed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            if await homeCoordinator.hasToggledOpenAppWindows() {
                return AccessibilityGuidanceContext(surface: .openAppsWindowControlUsed)
            }
            return AccessibilityGuidanceContext(surface: .none)
        }

        let route: CommandRoute
        if let queryState = lastNormalizedQueryState, queryState.raw == searchBar.stringValue {
            route = queryState.commandRoute
        } else {
            route = viewModel.commandRouter.route(raw: searchBar.stringValue)
        }
        if case .targeted(let module, _, _) = route,
           AccessibilityGuidancePolicy.isGuidanceModule(module) {
            return AccessibilityGuidanceContext(surface: .targetedModule(module))
        }
        return AccessibilityGuidanceContext(surface: .none)
    }

    private func stabilizePanelContentLayout() {
        LauncherInPanelLayout.stabilizePanel(from: searchBar)
    }

    private func applyRestore(moduleRaw: String?, query: String, translateSource: String, translateOutput: String) {
        sessionStore.suppressPersistence = true
        defer { sessionStore.suppressPersistence = false }
        switch sessionStore.restoreDecision(
            moduleRaw: moduleRaw,
            query: query,
            translateSource: translateSource,
            translateOutput: translateOutput
        ) {
        case .openModule(let moduleID, _, _):
            openModuleDetail(for: moduleID)
        case .restoreQuery(let query):
            showHome(focusSearch: false, persist: false)
            searchBar.stringValue = query
            handleTextChange(query)
        case .showHome:
            break
        }
    }

    private func apply(snapshot: ResultSnapshot) {
        guard !launcherEnvironment.isLauncherQueryEmpty else { return }
        LauncherPerfCounters.increment(.snapshotApply)
        contentCoordinator.apply(snapshot: snapshot)
        preferBareOpenDetailRowSelection()
        syncPerformanceStripVisibility()
        syncRowActionHint()
        if let paintMs = LatencyTracker.shared.markFirstPaint() {
            LatencyTelemetry.report(p95Milliseconds: paintMs)
        }
        stabilizePanelContentLayout()
        refreshPermissionBannerCoalesced()
    }

    private func enqueueSnapshotApply(_ snapshot: ResultSnapshot) {
        snapshotApplyCoalescer.enqueue(snapshot)
    }

    private func flushPendingSnapshotApplyNow() {
        snapshotApplyCoalescer.flushNow()
    }

    private func cancelPendingSnapshotApply() {
        snapshotApplyCoalescer.cancel()
    }

    private func flushPendingSnapshotApply() {
        snapshotApplyCoalescer.flushNow()
    }

    private func syncPerformanceStripVisibility() {
        performanceStrip.setContentVisible(true)
    }

    private func enterDetailContext(moduleTitle: String) {
        searchBar.beginDetailMode(moduleTitle: moduleTitle)
        hintBar.setContext(.detail)
    }

    private func activateReturn() {
        if !modulesReady {
            commandHintBar.showStatus(LauncherStatusMessages.modulesLoading)
            return
        }
        if actionPanel.isVisible {
            actionPanel.activateSelection()
            return
        }
        if performBareCommandAction() { return }

        // Snippet trigger expansion: if the raw query exactly matches a snippet
        // trigger, expand and paste it without navigating to results.
        let raw = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if !raw.isEmpty, !contentCoordinator.showingDetail {
            let route = viewModel.commandRouter.route(raw: raw)
            if case .globalSearch = route {
                Task {
                    if let snippet = await launcherEnvironment.snippetsModule.snippetForTrigger(raw) {
                        let outcome = (try? await launcherEnvironment.snippetsModule.insertSnippet(id: snippet.id)) ?? .copiedOnly
                        launcherEnvironment.showStatus(LauncherStatusMessages.message(for: outcome))
                        await MainActor.run { self.onActionDismiss() }
                        return
                    }
                    await MainActor.run { self.activateSelectedOrShowNoResults() }
                }
                return
            }
        }

        activateSelectedOrShowNoResults()
    }

    private func activateSelectedOrShowNoResults() {
        if contentCoordinator.currentItems.isEmpty {
            let raw = searchBar.stringValue
            let route = viewModel.commandRouter.route(raw: raw)
            let message = SearchEmptyState.message(
                for: route,
                query: raw,
                registry: viewModel.commandRouter.registry
            )
            commandHintBar.showStatus(message)
            return
        }
        if !contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex) {
            commandHintBar.showStatus(LauncherStatusMessages.noResultsYet)
            return
        }
        activateSelectedItem()
    }

    @discardableResult
    private func performBareCommandAction() -> Bool {
        let raw = searchBar.stringValue
        guard viewModel.commandRouter.isBareOpenDetailReturn(raw: raw) else { return false }
        let route = viewModel.commandRouter.route(raw: raw)
        guard case .targeted(let module, _, let payload) = route else { return false }

        viewModel.recordExecutedCommand(for: raw)
        if module == .snippets {
            let lower = payload.lowercased()
            if lower == "new" {
                LauncherSharedState.pendingSnippetDraft = SnippetDraft(title: "Untitled", content: "")
            } else if lower.hasPrefix("new ") {
                let title = String(payload.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
                LauncherSharedState.pendingSnippetDraft = SnippetDraft(
                    title: title.isEmpty ? "Untitled" : title,
                    content: ""
                )
            }
            openModuleDetail(for: module)
            return true
        }
        var detailPayload: Data?
        if module == .wordbook,
           payload.compare("review", options: .caseInsensitive) == .orderedSame {
            detailPayload = try? ModuleActionCoding.encode(WordbookAction.review)
        }
        openModuleDetail(for: module, payload: detailPayload)
        return true
    }

    private func syncRowActionHint() {
        syncKeyHints()
        guard contentCoordinator.showingResults || !contentCoordinator.showingDetail else {
            commandHintBar.setReturnAction(nil)
            return
        }
        if viewModel.commandRouter.isBareOpenDetailReturn(raw: searchBar.stringValue) {
            commandHintBar.setReturnAction("Open detail")
            return
        }
        if let item = contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex] {
            commandHintBar.setReturnAction(item.returnHint)
        } else {
            commandHintBar.setReturnAction(nil)
        }
    }

    private func syncKeyHints() {
        if contentCoordinator.showingDetail {
            hintBar.setContext(.detail)
            return
        }
        let context: LauncherHintContext = contentCoordinator.showingResults ? .results : .home
        let item = contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex]
        hintBar.setContext(context, selectedItem: item)
    }

    private func moduleDetailPresentation() -> ModuleDetailPresentation {
        .rightColumn
    }

    private func usesColumnSplitLayout() -> Bool {
        let trimmed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty { return false }
        if contentCoordinator.showingDetail { return true }
        return !contentCoordinator.showingResults
    }

    private func listHoldsKeyboardFocus() -> Bool {
        guard let responder = listView.window?.firstResponder else { return false }
        if responder === listView { return true }
        guard let view = responder as? NSView else { return false }
        return view.isDescendant(of: listView)
    }

    private func syncSplitLayout() {
        let columnSplit = usesColumnSplitLayout()
        if columnSplit {
            LauncherInPanelLayout.ensureHomeSplitPanelSize(from: searchBar)
        }

        let rightPane: LauncherSplitRightPane
        if columnSplit, contentCoordinator.showingDetail {
            rightPane = .detail
        } else if columnSplit {
            rightPane = .guide
        } else {
            rightPane = .hidden
        }

        let layoutChanged = lastSplitLayoutState.map { $0 != (columnSplit, rightPane) } ?? true
        lastSplitLayoutState = (columnSplit, rightPane)

        homeSplitLayout.setColumnSplitActive(columnSplit)
        homeSplitLayout.setRightPane(rightPane)

        if columnSplit, rightPane == .guide {
            let commands = viewModel.commandRouter.registry.discoverableCommands
            let catalogIDs = commands.map(\.id)
            if catalogIDs != lastAppliedGuideCatalogIDs {
                homeSplitLayout.guidePane.applyCatalog(commands)
                lastAppliedGuideCatalogIDs = catalogIDs
            }
        }

        if layoutChanged {
            stabilizePanelContentLayout()
        }
    }

    private func activateSelectedItem() {
        guard contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex) else { return }
        handleRun(contentCoordinator.currentItems[contentCoordinator.selectedIndex])
    }

    private func handleRun(_ item: ResultItem) {
        guard prepareConfirmedAction(item.primaryAction, for: item) else { return }
        executeRun(for: item)
    }

    private func executeRun(for item: ResultItem) {
        switch LauncherKeyRouter.resolveRun(item: item) {
        case .toggleOpenAppWindows(let bundleID):
            toggleOpenAppWindows(bundleID: bundleID)
        case .runItem(let resolved):
            viewModel.recordExecutedCommand(for: searchBar.stringValue)
            dispatchAction(resolved.primaryAction, for: resolved)
        default:
            viewModel.recordExecutedCommand(for: searchBar.stringValue)
            dispatchAction(item.primaryAction, for: item)
        }
    }

    private func dispatchAction(_ action: Action, for item: ResultItem) {
        switch action.kind {
        case .noop:
            focusSearchField()
        case .replaceQuery(let query):
            searchBar.stringValue = query
            handleTextChange(query)
            focusSearchField()
        case .openModuleDetail(let moduleID, let payload):
            openModuleDetail(for: moduleID, payload: payload)
        case .translateText(let text):
            openTranslateDetail(with: text)
        case .custom(let payload, let handler):
            if handler == .workbench {
                if let captureAction = try? ModuleActionCoding.decode(WorkbenchCaptureAction.self, from: payload) {
                    runWorkbenchCapture(captureAction, for: item)
                    return
                }
                if let commandAction = try? ModuleActionCoding.decode(WorkbenchCommandAction.self, from: payload) {
                    runWorkbenchCommand(commandAction, for: item)
                    return
                }
                if let entityAction = try? ModuleActionCoding.decode(WorkbenchEntityAction.self, from: payload) {
                    runWorkbenchEntity(entityAction, for: item)
                    return
                }
            }
            if handler == .notes,
               let notesAction = try? ModuleActionCoding.decode(NotesAction.self, from: payload) {
                switch notesAction {
                case .captureToDaily(let text):
                    runNotesCapture(text: text, for: item)
                    return
                case .open(let path):
                    runNotesOpen(path: path, action: action, for: item)
                    return
                default:
                    break
                }
            }
            runWithFeedback(action: action, for: item)
        default:
            runWithFeedback(action: action, for: item)
        }
    }

    private func runWorkbenchCapture(_ action: WorkbenchCaptureAction, for item: ResultItem) {
        switch action {
        case .prepareDraft(let source, let target):
            runWorkbenchPrepareDraft(source: source, target: target, for: item)
        case .resumeActivity(let entryID):
            runWorkbenchResumeActivity(entryID: entryID, for: item)
        }
    }

    func runWorkbenchCaptureFromDetail(source: WorkbenchCaptureSource, target: WorkbenchCaptureTarget) {
        let item = ResultItem(
            id: ResultID(module: target.moduleID, key: "detail.capture"),
            title: "Project capture",
            titleAttributed: AttributedString("Project capture"),
            subtitle: nil,
            icon: .symbol("plus.circle"),
            primaryAction: Action(
                id: ActionID(module: target.moduleID, key: "detail.capture"),
                title: "Project capture",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: 0),
        )
        runWorkbenchPrepareDraft(source: source, target: target, for: item)
    }

    func runWorkspaceRowActionFromDetail(_ action: CurrentProjectWorkspaceRowAction) {
        switch action {
        case .resumeActivity(let entryID):
            runWorkbenchResumeActivity(entryID: entryID, for: ResultItem(
                id: ResultID(module: .workbench, key: "detail.resume.\(entryID.uuidString)"),
                title: "Resume activity",
                titleAttributed: AttributedString("Resume activity"),
                subtitle: nil,
                icon: .symbol("clock.arrow.circlepath"),
                primaryAction: Action(
                    id: ActionID(module: .workbench, key: "detail.resume"),
                    title: "Resume",
                    kind: .noop
                ),
                rankingHints: RankingHints(basePriority: 0),
            ))
        case .openModule(let moduleID):
            openModuleDetail(for: moduleID)
        case .replaceQuery(let text):
            searchBar.stringValue = text
            lastSyncedQuery = ""
            handleTextChange(text)
        case .openNotePath(let path):
            openNote(at: path, for: ResultItem(
                id: ResultID(module: .notes, key: "detail.note"),
                title: "Open note",
                titleAttributed: AttributedString("Open note"),
                subtitle: nil,
                icon: .symbol("note.text"),
                primaryAction: Action(
                    id: ActionID(module: .notes, key: "detail.note"),
                    title: "Open note",
                    kind: .noop
                ),
                rankingHints: RankingHints(basePriority: 0),
            ))
        case .openLinked(let linkID):
            runOpenLinkedEntity(linkID: linkID)
        case .status(let message):
            commandHintBar.showStatus(message)
        }
    }

    private func runWorkbenchEntity(_ action: WorkbenchEntityAction, for item: ResultItem) {
        switch action {
        case .openLinked(let linkID):
            runOpenLinkedEntity(linkID: linkID)
        case .openActivityEntry(let entryID):
            runOpenActivityEntry(entryID: entryID, for: item)
        case .showStatus(let message):
            commandHintBar.showStatus(message)
        }
    }

    func runOpenActivityEntry(entryID: UUID, for item: ResultItem?) {
        Task {
            let activities = await WorkbenchActivityStore.shared.allEntries()
            guard let entry = activities.first(where: { $0.id == entryID }) else {
                await MainActor.run { commandHintBar.showStatus(LauncherStatusMessages.activityNoLongerAvailable) }
                return
            }
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            guard enabled.contains(entry.moduleID) else {
                await MainActor.run { commandHintBar.showStatus(LauncherStatusMessages.moduleDisabledInSettings) }
                return
            }
            let rowAction = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry)
            await MainActor.run {
                runWorkspaceRowActionFromDetail(rowAction)
                persistResumeState(translateContent: nil)
            }
        }
    }

    func runOpenLinkedEntity(linkID: UUID) {
        Task {
            let links = await WorkbenchLinkStore.shared.allLinks()
            guard let link = links.first(where: { $0.id == linkID }) else {
                await MainActor.run { commandHintBar.showStatus(LauncherStatusMessages.linkedItemNoLongerAvailable) }
                return
            }
            let entry: WorkbenchActivityEntry?
            if let entryID = link.activityEntryID {
                let activities = await WorkbenchActivityStore.shared.allEntries()
                entry = activities.first { $0.id == entryID }
            } else {
                entry = nil
            }
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            guard enabled.contains(link.entityRef.moduleID) else {
                await MainActor.run { commandHintBar.showStatus(LauncherStatusMessages.moduleDisabledInSettings) }
                return
            }
            let rowAction = WorkbenchLinkedEntityOpenPlanner.rowAction(for: link, entry: entry)
            await MainActor.run {
                runWorkspaceRowActionFromDetail(rowAction)
                persistResumeState(translateContent: nil)
            }
        }
    }

    private func runWorkbenchPrepareDraft(
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        for item: ResultItem
    ) {
        Task {
            let signals = await panelSignalsLoader.load()
            let followUp = workbenchFollowUp(for: target)
            let runner = WorkbenchCaptureRunner()
            guard let result = await runner.runCapture(
                source: source,
                target: target,
                enabledModuleIDs: signals.enabledModuleIDs,
                pinnedModuleIDs: signals.pinnedModuleIDs,
                clipboardPreview: signals.clipboardPreview,
                selectionText: signals.selectionText,
                attribution: WorkbenchCaptureAttribution(sourceKind: .home, followUp: followUp)
            ) else {
                await MainActor.run {
                    if !signals.enabledModuleIDs.contains(target.moduleID) {
                        commandHintBar.showStatus(LauncherStatusMessages.moduleDisabledInSettings)
                    } else {
                        commandHintBar.showStatus(LauncherStatusMessages.nothingToCapture)
                    }
                }
                return
            }
            await MainActor.run {
                applyWorkbenchCaptureResult(result, for: item)
                persistResumeState(translateContent: nil)
            }
        }
    }

    private func runWorkbenchResumeActivity(entryID: UUID, for item: ResultItem) {
        Task {
            let activities = await WorkbenchActivityStore.shared.allEntries()
            guard let entry = activities.first(where: { $0.id == entryID }) else {
                await MainActor.run { commandHintBar.showStatus(LauncherStatusMessages.activityNoLongerAvailable) }
                return
            }
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            guard enabled.contains(entry.moduleID) else {
                await MainActor.run { commandHintBar.showStatus(LauncherStatusMessages.moduleDisabledInSettings) }
                return
            }
            await MainActor.run {
                resumeFromActivityEntry(entry)
                persistResumeState(translateContent: nil)
            }
        }
    }

    private func resumeFromActivityEntry(_ entry: WorkbenchActivityEntry) {
        if let payload = entry.resumablePayload {
            applyResumePayload(payload)
            return
        }
        guard let resumeRef = entry.resumeRef else { return }
        let resume = LauncherResumeStore.load()
        switch resumeRef.kind {
        case .snippetDraft:
            if let data = resume.snippetDraftJSON,
               let draft = try? JSONDecoder().decode(SnippetDraft.self, from: data) {
                LauncherSharedState.pendingSnippetDraft = draft
            }
            openModuleDetail(for: .snippets)
        case .quicklinkDraft:
            if let data = resume.quicklinkDraftJSON,
               let draft = try? JSONDecoder().decode(URLQuicklinkDraft.self, from: data) {
                LauncherSharedState.pendingQuicklinkDraft = draft
            }
            openModuleDetail(for: .quicklinks)
        case .todoCapture:
            let text = resume.todoCaptureText ?? entry.preview ?? ""
            searchBar.stringValue = TodoModule.resumeQuery(forCapture: text)
            lastSyncedQuery = ""
            handleTextChange(searchBar.stringValue)
        case .noteAction:
            commandHintBar.showStatus("Note capture already applied")
        }
    }

    private func applyResumePayload(_ payload: WorkbenchActivityResumePayload) {
        switch payload {
        case .snippetDraft(let data):
            if let draft = try? JSONDecoder().decode(SnippetDraft.self, from: data) {
                LauncherSharedState.pendingSnippetDraft = draft
            }
            openModuleDetail(for: .snippets)
        case .quicklinkDraft(let data):
            if let draft = try? JSONDecoder().decode(URLQuicklinkDraft.self, from: data) {
                LauncherSharedState.pendingQuicklinkDraft = draft
            }
            openModuleDetail(for: .quicklinks)
        case .todoCapture(let text):
            searchBar.stringValue = TodoModule.resumeQuery(forCapture: text)
            lastSyncedQuery = ""
            handleTextChange(searchBar.stringValue)
        case .noteReference(let path, _):
            openNote(at: path, for: nil)
        }
    }

    private func openNote(at path: String, for item: ResultItem?) {
        let payload = (try? ModuleActionCoding.encode(NotesAction.open(path: path))) ?? Data()
        let resultItem = item ?? ResultItem(
            id: ResultID(module: .notes, key: "workbench.note.\(path)"),
            title: "Open note",
            titleAttributed: AttributedString("Open note"),
            subtitle: nil,
            icon: .symbol("note.text"),
            primaryAction: Action(
                id: ActionID(module: .notes, key: "workbench.note"),
                title: "Open note",
                kind: .custom(payload: payload, handler: .notes)
            ),
            rankingHints: RankingHints(basePriority: 0),
        )
        let action = Action(
            id: ActionID(module: .notes, key: "workbench.note"),
            title: "Open note",
            kind: .custom(payload: payload, handler: .notes)
        )
        runNotesOpen(path: path, action: action, for: resultItem)
    }

    private func applyWorkbenchCaptureResult(_ result: WorkbenchCaptureResult, for item: ResultItem) {
        switch result.target {
        case .noteDraft:
            if let payload = result.actionPayload {
                let act = Action(
                    id: item.primaryAction.id,
                    title: item.primaryAction.title,
                    kind: .custom(payload: payload, handler: .notes)
                )
                runWithFeedback(action: act, for: item)
            }
        case .todoDraft:
            searchBar.stringValue = TodoModule.resumeQuery(forCapture: result.preview)
            lastSyncedQuery = ""
            handleTextChange(searchBar.stringValue)
        case .snippetDraft, .quicklinkDraft, .projectSnippetDraft:
            openModuleDetail(for: result.moduleID, payload: result.openDetailPayload)
        }
    }

    private func workbenchFollowUp(for target: WorkbenchCaptureTarget) -> WorkbenchCaptureFollowUp {
        switch target {
        case .noteDraft: .runNotesAction
        case .todoDraft: .replaceQuery
        case .snippetDraft, .quicklinkDraft, .projectSnippetDraft: .openDetail
        }
    }

    private func runWorkbenchCommand(_ action: WorkbenchCommandAction, for item: ResultItem) {
        guard case .execute(let commandID) = action else { return }
        Task {
            let signals = await panelSignalsLoader.load()
            let outcome = await workbenchCommandExecutor.handle(
                commandID: commandID,
                enabledModuleIDs: signals.enabledModuleIDs,
                pinnedModuleIDs: signals.pinnedModuleIDs,
                clipboardPreview: signals.clipboardPreview,
                selectionText: signals.selectionText
            )
            await MainActor.run {
                applyWorkbenchCommandOutcome(outcome)
                persistResumeState(translateContent: nil)
            }
        }
    }

    private func runNotesCapture(text: String, for item: ResultItem) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            commandHintBar.showStatus(LauncherStatusMessages.nothingToCapture)
            return
        }
        Task {
            let outcome = await NotesCaptureHelper.appendToDailyNote(trimmed, openAfterCapture: false)
            guard case .appended = outcome else { return }
            saveModuleRoundTripResume(module: .notes)
            try? await Task.sleep(for: .milliseconds(900))
            await MainActor.run { self.onActionDismiss() }
        }
    }

    private func runNotesOpen(path _: String, action: Action, for item: ResultItem) {
        Task {
            await MainActor.run { self.onActionDismiss() }
            await actionExecutor.run(action, for: item)
        }
    }

    private func runWithFeedback(action: Action, for item: ResultItem) {
        if let feedback = LauncherActionFeedback.feedback(for: action) {
            commandHintBar.showStatus(feedback.message)
            Task {
                let result = await actionExecutor.run(action, for: item)
                guard result.succeeded else {
                    await MainActor.run {
                        let message = result.userFacingMessage ?? LauncherStatusMessages.operationFailed
                        self.commandHintBar.showStatus(message)
                    }
                    return
                }
                if feedback.delayDismiss {
                    try? await Task.sleep(for: .milliseconds(900))
                    await MainActor.run { self.onActionDismiss() }
                }
            }
        } else {
            onActionDismiss()
            Task {
                await actionExecutor.run(action, for: item)
            }
        }
    }

    private func saveModuleRoundTripResume(module: ModuleIdentifier, query: String? = nil) {
        let resolved = LauncherModuleResumeQuery.normalizedQuery(
            for: module,
            raw: query ?? searchBar.stringValue
        )
        LauncherResumeStore.save(LauncherResumeState(moduleRaw: module.rawValue, query: resolved))
    }

    private func run(item: ResultItem) {
        dispatchAction(item.primaryAction, for: item)
    }

    private func runAction(_ action: Action, for item: ResultItem) {
        guard prepareConfirmedAction(action, for: item) else { return }
        if action.id == item.primaryAction.id {
            executeRun(for: item)
            return
        }
        dispatchAction(action, for: item)
    }

    private func prepareConfirmedAction(_ action: Action, for item: ResultItem) -> Bool {
        if action.confirmation == .requireSecondModifier, !actionPanel.isVisible {
            openActionPanel(for: item)
            commandHintBar.showStatus("Confirm \(action.title) in the action panel")
            return false
        }
        if action.confirmation != .none, !confirmDestructiveAction(action, for: item) {
            return false
        }
        return true
    }

    private func confirmDestructiveAction(_ action: Action, for item: ResultItem) -> Bool {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = action.title
        alert.informativeText = "Confirm \(action.title.lowercased()) for \"\(item.title)\"? This cannot be undone."
        alert.addButton(withTitle: action.title)
        alert.addButton(withTitle: "Cancel")
        return alert.runModal() == .alertFirstButtonReturn
    }

    private func preferBareOpenDetailRowSelection() {
        guard viewModel.commandRouter.isBareOpenDetailReturn(raw: searchBar.stringValue) else { return }
        guard case .targeted(let module, _, _) = viewModel.commandRouter.route(raw: searchBar.stringValue) else { return }
        guard let index = contentCoordinator.currentItems.firstIndex(where: {
            $0.id.module == module && $0.id.key == "open-detail"
        }) else { return }
        contentCoordinator.updateSelection(to: index)
    }

    private func openTranslateDetail(with text: String) {
        contentCoordinator.pendingTranslateText = text
        if contentCoordinator.showingDetail,
           let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
            translate.prefill(text: text, autoTranslate: true)
            contentCoordinator.pendingTranslateText = nil
            return
        }
        openModuleDetail(for: .translate)
    }

    private func openActionPanel(for item: ResultItem? = nil) {
        let target = item ?? contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex]
        guard let target else { return }
        guard !target.secondaryActions.isEmpty else {
            commandHintBar.showStatus(LauncherStatusMessages.noActionPanelActions)
            return
        }
        let anchor = listView.selectedRowAnchorView() ?? hintBar
        actionPanel.present(item: target, relativeTo: anchor)
    }

    private func handleKeyCommand(_ command: LumaSearchBar.KeyCommand) -> Bool {
        if case .backtab = command {
            if actionPanel.isVisible {
                actionPanel.dismiss()
                return true
            }
            return false
        }
        if case .commandReturn = command {
            guard !actionPanel.isVisible, !contentCoordinator.showingDetail,
                  let item = contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex] else { return false }
            guard let secondary = item.secondaryActions.first else {
                commandHintBar.showStatus(LauncherStatusMessages.noAlternateActions)
                return true
            }
            runAction(secondary, for: item)
            return true
        }
        if actionPanel.isVisible {
            if case .commandNumber(let number) = command {
                actionPanel.activateIndex(number - 1)
                return true
            }
            if case .up = command {
                actionPanel.moveSelection(delta: -1)
                return true
            }
            if case .down = command {
                actionPanel.moveSelection(delta: 1)
                return true
            }
            let outcome = LauncherKeyRouter.route(
                command: command.launcherKeyCommand,
                mode: .results,
                itemCount: actionPanel.actionCount,
                actionPanelVisible: true
            )
            if outcome == .dismissActionPanel {
                actionPanel.dismiss()
                return true
            }
            return true
        }
        let mode: LauncherContentMode
        if contentCoordinator.showingDetail, !listHoldsKeyboardFocus() {
            mode = .detail
        } else if contentCoordinator.showingResults
            || !searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            mode = .results
        } else {
            mode = .home
        }
        let outcome = LauncherKeyRouter.route(
            command: command.launcherKeyCommand,
            mode: mode,
            itemCount: contentCoordinator.currentItems.count,
            actionPanelVisible: actionPanel.isVisible
        )
        switch outcome {
        case .handled: return true
        case .openActionPanel: openActionPanel(); return true
        case .moveSelection(let delta):
            let next = contentCoordinator.selectedIndex + delta
            contentCoordinator.updateSelection(to: min(max(0, next), max(0, contentCoordinator.currentItems.count - 1)))
            return true
        case .jumpToFlatIndex(let index):
            contentCoordinator.updateSelection(to: index)
            if let item = contentCoordinator.currentItems[safe: index] { handleRun(item) }
            return true
        case .runItem(let item): run(item: item); return true
        case .toggleOpenAppWindows:
            return true
        case .dismissActionPanel:
            actionPanel.dismiss()
            return true
        case .passthrough: return false
        }
    }
}

private extension LumaSearchBar.KeyCommand {
    var launcherKeyCommand: LauncherKeyCommand {
        switch self {
        case .up: .up
        case .down: .down
        case .tab: .tab
        case .backtab: .backtab
        case .actionPanel: .actionPanel
        case .commandReturn: .commandReturn
        case .commandNumber(let n): .commandNumber(n)
        }
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
