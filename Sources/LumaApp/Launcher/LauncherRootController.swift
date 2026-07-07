import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class LauncherRootController: LauncherDetailPresenting {
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
    private var workbenchPreviewTask: Task<Void, Never>?
    private var querySyncTimer: Timer?
    private var lastSyncedQuery = ""
    var lastSplitLayoutState: (columnSplit: Bool, rightPane: LauncherSplitRightPane)?
    private var lastResultsRouteKind: QueryView.ResultsRouteKind?
    private var lastQueryView: QueryView?
    private var permissionRefreshTask: Task<Void, Never>?
    private var lastAppliedGuideCatalogIDs: [String] = []
    private let taskRegistry = LauncherTaskRegistry()
    private var homeRefreshGeneration = CancellationGeneration()
    private var sessionState = LauncherSessionState()
    private lazy var snapshotPipeline = LauncherSnapshotApplyPipeline(
        contentCoordinator: contentCoordinator,
        isPanelActive: { [weak self] in self?.isPanelActiveForQueryApply ?? false },
        isQueryEmpty: { [weak self] in self?.launcherEnvironment.isLauncherQueryEmpty ?? true },
        onApplied: { [weak self] in
            self?.onSnapshotApplied()
        }
    )
    private lazy var detailLifecycle = LauncherDetailLifecycleController(
        contentCoordinator: contentCoordinator,
        homeSplitLayout: homeSplitLayout,
        searchBar: searchBar,
        usesColumnSplitLayout: { [weak self] in self?.usesColumnSplitLayout() ?? false },
        discoverableCommands: { [weak self] in
            self?.viewModel.commandRouter.registry.discoverableCommands ?? []
        },
        enabledModuleIDs: { [weak self] in
            self?.cachedEnabledModuleIDs ?? ModuleRegistry.defaultEnabledModuleIDs
        }
    )
    private lazy var detailPresenter = LauncherDetailPresenter(
        host: self,
        config: config,
        sessionStore: sessionStore,
        taskRegistry: taskRegistry,
        detailLifecycle: detailLifecycle,
        launcherEnvironment: launcherEnvironment,
        homeCoordinator: homeCoordinator,
        contentCoordinator: contentCoordinator,
        searchBar: searchBar,
        hintBar: hintBar,
        homeSplitLayout: homeSplitLayout
    )
    private var restoreGeneration = CancellationGeneration()
    private var querySyncGraceUntil: ContinuousClock.Instant?
    var isPanelActiveForQueryApply = false
    private var cachedEnabledModuleIDs = ModuleRegistry.defaultEnabledModuleIDs

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
        Task { await self.refreshEnabledModuleCache() }
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
            self?.handleTextChange(text)
        }
        searchBar.onCompositionActive = { [weak self] in
            self?.noteSearchTextChangedForQuerySync()
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
        let snapshot = ResultSnapshot(querySequence: 0, items: items)
        await MainActor.run { [weak self] in
            guard let self, self.isPanelActiveForQueryApply else { return }
            guard !Task.isCancelled else { return }
            self.enqueueSnapshotApply(snapshot)
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
            transitionToResultsMode(clearStaleContent: false)
            listView.isHidden = false
            enqueueSnapshotApply(ResultSnapshot(querySequence: 0, items: []))
            return true
        case .openDetail(let moduleID, let payload):
            openModuleDetail(for: moduleID, payload: payload)
            return true
        case .replaceQuery(let text):
            if let replaceQueryHandler {
                replaceQueryHandler(text)
            } else {
                searchBar.stringValue = text
                resetSyncedQueryForRestore()
                handleTextChange(text)
            }
            return true
        case .runAction(let kind):
            let action = Action(
                id: ActionID(module: .notes, key: "workbench.capture"),
                title: "Capture",
                kind: kind
            )
            performActionThenDismiss(action: action, for: ResultItem(
                id: ResultID(module: .notes, key: "workbench.capture"),
                title: "Capture",
                titleAttributed: AttributedString("Capture"),
                icon: .symbol("square.and.arrow.down"),
                primaryAction: action,
                secondaryActions: [],
                rankingHints: RankingHints(basePriority: 0)
            ))
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
        syncQueryBaselineFromSearchField()
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
        resetSyncedQueryForRestore()
        commandHintBar.apply(nil)
        recordQueryEmptyState()
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

    var lastRenderedHomeGeneration: UInt64 = 0

    func refreshHome(
        preserveListSelection: Bool = false,
        force: Bool = false,
        intent: LauncherHomeRefreshIntent = .visibleRepaint
    ) {
        homeRefreshTask?.cancel()
        let generation = homeRefreshGeneration.current
        homeRefreshTask = Task { [weak self] in
            guard let self else { return }
            if !force {
                let homeGen = await self.homeCoordinator.currentSnapshotGeneration()
                if homeGen == self.lastRenderedHomeGeneration,
                   await self.homeCoordinator.cachedSnapshotIfAvailable() != nil {
                    if LauncherHomeRefreshRepaintPolicy.shouldCloseHotkeyLatencyOnCacheHit(
                        intent: intent,
                        isPanelActiveForQueryApply: self.isPanelActiveForQueryApply
                    ) {
                        await MainActor.run {
                            _ = HomeLatencyTracker.markHomeRendered()
                        }
                    }
                    return
                }
            }
            let snapshot = await self.homeCoordinator.snapshot()
            guard !Task.isCancelled else { return }
            var didRepaint = false
            await MainActor.run {
                guard self.homeRefreshGeneration.isCurrent(generation) else { return }
                let guards = LauncherHomeRefreshRepaintPolicy.VisibleRepaintGuards(
                    queryTrimmedEmpty: self.searchBar.stringValue
                        .trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                    showingDetail: self.contentCoordinator.showingDetail,
                    showingResults: self.contentCoordinator.showingResults,
                    isLauncherQueryEmpty: self.launcherEnvironment.isLauncherQueryEmpty
                )
                guard LauncherHomeRefreshRepaintPolicy.shouldRepaintHome(intent: intent, guards: guards) else {
                    return
                }
                if guards.showingDetail {
                    self.contentCoordinator.showHome(snapshot, preserveSelection: true)
                    self.syncRowActionHint()
                } else {
                    self.contentCoordinator.showHome(
                        snapshot,
                        preserveSelection: preserveListSelection
                    )
                    self.syncSplitLayout()
                    self.syncRowActionHint()
                    if self.isPanelActiveForQueryApply {
                        _ = HomeLatencyTracker.markHomeRendered()
                    }
                }
                didRepaint = true
            }
            if LauncherHomeRefreshRepaintPolicy.shouldAdvanceRenderedGeneration(
                intent: intent,
                didRepaint: didRepaint
            ) {
                self.lastRenderedHomeGeneration = await homeCoordinator.currentSnapshotGeneration()
            }
        }
        if let homeRefreshTask {
            taskRegistry.register(key: "homeRefresh", task: homeRefreshTask)
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
        resetSyncedQueryForRestore()
        commandHintBar.apply(nil)
        recordQueryEmptyState()
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

    private func shouldClearStaleResults(for kind: QueryView.ResultsRouteKind) -> Bool {
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
        activatePanelForQueryApply()
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
        resetSyncedQueryForRestore()
        handleTextChange(searchBar.stringValue)
    }

    func cancelPendingRestore() {
        restoreGeneration.bump()
        snapshotPipeline.cancelPending()
    }

    /// Cancels in-flight async launcher work without changing panel-active apply policy or session panel state.
    func cancelLauncherAsyncWork() {
        let shouldTearDownDetailClose = detailLifecycle.consumeDetailCloseCrossfadeInFlight()
        detailLifecycle.invalidateCrossfadeCompletions()
        if shouldTearDownDetailClose {
            detailLifecycle.tearDownAfterGuideCrossfadeIfNeeded()
        }
        detailLifecycle.cancelPendingPresentation()
        homeRefreshGeneration.bump()
        viewModel.cancel()
        snapshotPipeline.cancelPending()
        homeRefreshTask?.cancel()
        homeRefreshTask = nil
        permissionRefreshTask?.cancel()
        permissionRefreshTask = nil
        workbenchPreviewTask?.cancel()
        workbenchPreviewTask = nil
        taskRegistry.cancelAll()
        actionPanel.dismiss()
        syncKeyHints()
        syncPerformanceStripVisibility()
    }

    /// Cancels async work and marks the panel inactive for snapshot apply (panel hide only).
    func cancelActiveQueryAndSnapshotApply() {
        cancelLauncherAsyncWork()
        markPanelInactiveForSnapshotApply()
        LauncherPerfCounters.increment(.queryCancelOnHide)
    }

    func handleModulesDisabled(removed: Set<ModuleIdentifier>) {
        cancelLauncherAsyncWork()
        viewModel.invalidateSnapshotCache()
        if let current = contentCoordinator.currentDetailModuleID, removed.contains(current) {
            exitDetailFromChrome()
        }
        launcherEnvironment.evictDetailModules(removed)
        invalidatePanelSignalsCache()
        Task { await refreshEnabledModuleCache() }
    }

    private func refreshEnabledModuleCache() async {
        cachedEnabledModuleIDs = await config.enabledModules() ?? ModuleRegistry.defaultEnabledModuleIDs
        lastAppliedGuideCatalogIDs = []
        await MainActor.run { syncSplitLayout() }
    }

    func activatePanelForQueryApply() {
        markPanelActiveForSnapshotApply()
    }

    func applySessionEvent(_ event: LauncherSessionEvent) {
        // Partial production wiring — see LAUNCHER_SESSION_STATE_AUDIT.md (4/11 events).
        let effects = sessionState.apply(event)
        LauncherSessionEffectApplier.apply(effects, environment: sessionEffectEnvironment())
    }

    private func sessionEffectEnvironment() -> LauncherSessionEffectApplier.Environment {
        LauncherSessionEffectApplier.Environment(
            cancelAllTasks: { [weak self] in self?.cancelLauncherAsyncWork() },
            clearDetailModeState: { [weak self] in
                self?.searchBar.cancelDetailMode()
            }
        )
    }

    private func onSnapshotApplied() {
        preferBareOpenDetailRowSelection()
        syncPerformanceStripVisibility()
        syncRowActionHint()
        if let paintMs = LatencyTracker.shared.markFirstPaint() {
            LatencyTelemetry.reportKeystrokeToPaint(paintMs)
        }
        stabilizePanelContentLayout()
        refreshPermissionBannerCoalesced()
    }

    func restoreLastSessionIfNeeded() {
        guard ProcessInfo.processInfo.environment["LUMA_QA"] != "1" else { return }
        if contentCoordinator.showingDetail {
            focusSearchField()
            return
        }
        guard searchBar.stringValue.isEmpty else { return }
        let generation = restoreGeneration.current
        Task {
            let persisted = await sessionStore.loadPersistedSession()
            await MainActor.run {
                guard self.restoreGeneration.isCurrent(generation) else { return }
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
            persistDetailForActionDismiss()
            return
        }
        resetLauncherStateAfterActionDismiss()
    }

    /// Detail stays mounted for session restore; action dismiss only flushes persisted session.
    private func persistDetailForActionDismiss() {
        saveCurrentSession()
    }

    private func resetLauncherStateAfterActionDismiss() {
        sessionStore.suppressPersistence = true
        searchBar.stringValue = ""
        recordQueryEmptyState()
        contentCoordinator.resetResults()
        viewModel.cancel()
        sessionStore.suppressPersistence = false
        saveHomeSession()
        persistResumeState(translateContent: nil)
        refreshHome()
    }

    func openModuleDetail(for moduleID: ModuleIdentifier, payload: Data? = nil) {
        detailPresenter.openModuleDetail(for: moduleID, payload: payload)
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
        applyDetailExitFromChrome()
    }

    /// User typed a new query while module detail was open — discard suspended query and search.
    private func dismissDetailForNewQuery() {
        searchBar.cancelDetailMode()
        closeDetail(animatedToGuide: false)
    }

    private func applyDetailExitFromChrome() {
        let outcome = LauncherDetailExitPlanner.outcome(
            showingDetail: contentCoordinator.showingDetail,
            suspendedQuery: searchBar.detailSuspendedQueryForPlanner,
            columnSplitActive: currentHomeSplitState().columnSplitActive
        )
        applyDetailExitOutcome(outcome)
    }

    private func applyDetailExitOutcome(_ outcome: LauncherDetailExitOutcome) {
        switch outcome {
        case .reenableSearchOnly:
            searchBar.reEnableSearchFieldIfNeeded()
            focusSearchField()
        case .restoreSuspendedQuery(let restored):
            _ = searchBar.endDetailMode()
            closeDetail(animatedToGuide: false)
            searchBar.stringValue = restored
            resetSyncedQueryForRestore()
            handleTextChange(restored)
            focusSearchField()
        case .returnToHome(let crossfadeToGuide):
            _ = searchBar.endDetailMode()
            closeDetail(animatedToGuide: crossfadeToGuide) { [weak self] in
                guard let self else { return }
                self.restoreHomeFromDetail(persist: true)
                self.focusSearchField()
            }
        }
    }

    func closeDetail(animatedToGuide: Bool = false, completion: (@MainActor () -> Void)? = nil) {
        Task { await launcherEnvironment.reserveDetailModule(nil) }
        if animatedToGuide, usesColumnSplitLayout() {
            lastAppliedGuideCatalogIDs = viewModel.commandRouter.registry.discoverableCommands.map(\.id)
        }
        detailLifecycle.onTearDown = { [weak self] in
            guard let self else { return }
            self.syncSplitLayout()
            self.lastSplitLayoutState = (self.usesColumnSplitLayout(), .guide)
            self.syncKeyHints()
            self.syncPerformanceStripVisibility()
            self.refreshPermissionBanner()
            applySessionEvent(.detailClosed)
        }
        detailLifecycle.closeDetail(animatedToGuide: animatedToGuide, completion: completion)
    }

    private func tearDownDetailAfterGuideCrossfade() {
        detailLifecycle.tearDownAfterGuideCrossfadeIfNeeded()
    }

    func showStatus(_ message: String) {
        commandHintBar.showStatus(message)
    }

    func handleTextChange(_ text: String) {
        guard text != lastSyncedQuery else { return }
        if LauncherActionPanelInvalidationPolicy.shouldDismissOnQueryChange(
            previousQuery: lastSyncedQuery,
            newQuery: text,
            actionPanelVisible: actionPanel.isVisible
        ) {
            actionPanel.dismiss()
        }
        let queryState = QueryView(raw: text, viewModel: viewModel)
        lastQueryView = queryState
        recordQueryTextChange(text, isEmpty: queryState.trimmed.isEmpty)
        searchBar.setPlaceholder(ModuleSearchHints.placeholder(for: text))
        commandHintBar.apply(queryState.hint, helpTrigger: queryState.helpTrigger)
        LauncherPerfCounters.increment(.layoutHint)
        sessionStore.saveSearchQuery(text)
        if queryState.trimmed.isEmpty {
            lastResultsRouteKind = .empty
            transitionToEmptyQueryHome()
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
            dismissDetailForNewQuery()
        }
        listView.isHidden = false
        listView.alphaValue = 1
        let clearStale = shouldClearStaleResults(for: queryState.resultsRouteKind)
        transitionToResultsMode(clearStaleContent: clearStale)
        lastResultsRouteKind = queryState.resultsRouteKind
        syncKeyHints()
        syncPerformanceStripVisibility()
        if queryState.workbenchRoute != .none {
            workbenchPreviewTask?.cancel()
            workbenchPreviewTask = Task { [weak self] in
                guard let self else { return }
                await self.previewWorkbenchCommand(route: queryState.workbenchRoute, queryText: text)
            }
            if let workbenchPreviewTask {
                taskRegistry.register(key: "workbenchPreview", task: workbenchPreviewTask)
            }
            return
        }
        let route = queryState.commandRoute
        if case .globalSearch = route, queryState.trimmed.count < 2 {
            transitionToClearedResultsList()
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
        if let permissionRefreshTask {
            taskRegistry.register(key: "permissionRefresh", task: permissionRefreshTask)
        }
    }

    func refreshPermissionBannerCoalesced() {
        permissionRefreshTask?.cancel()
        permissionRefreshTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(16))
            guard !Task.isCancelled else { return }
            await self?.refreshPermissionBannerNow()
        }
        if let permissionRefreshTask {
            taskRegistry.register(key: "permissionRefresh", task: permissionRefreshTask)
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

        let route = viewModel.commandRouter.route(raw: searchBar.stringValue)
        if case .targeted(let module, _, _) = route,
           AccessibilityGuidancePolicy.isGuidanceModule(module) {
            return AccessibilityGuidanceContext(surface: .targetedModule(module))
        }
        return AccessibilityGuidanceContext(surface: .none)
    }

    func stabilizePanelContentLayout() {
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
        snapshotPipeline.apply(snapshot: snapshot)
    }

    private func enqueueSnapshotApply(_ snapshot: ResultSnapshot) {
        snapshotPipeline.enqueue(snapshot)
    }

    private func cancelPendingSnapshotApply() {
        snapshotPipeline.cancelPending()
    }

    func syncPerformanceStripVisibility() {
        performanceStrip.setContentVisible(true)
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
                        do {
                            let outcome = try await launcherEnvironment.snippetsModule.insertSnippet(id: snippet.id)
                            launcherEnvironment.showStatus(LauncherStatusMessages.message(for: outcome))
                            if outcome == .pasted || outcome == .copiedOnly {
                                await MainActor.run { self.onActionDismiss() }
                            }
                        } catch {
                            await MainActor.run {
                                let mapped = ActionExecutionFailureMapper.message(for: error)
                                let message = mapped.message ?? LauncherStatusMessages.operationFailed
                                self.commandHintBar.showStatus(message)
                            }
                        }
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
        switch LauncherReturnActivationPolicy.outcome(
            itemCount: contentCoordinator.currentItems.count,
            selectedIndex: contentCoordinator.selectedIndex
        ) {
        case .showEmptyQueryMessage:
            let raw = searchBar.stringValue
            let route = viewModel.commandRouter.route(raw: raw)
            let message = SearchEmptyState.message(
                for: route,
                query: raw,
                registry: viewModel.commandRouter.registry
            )
            commandHintBar.showStatus(message)
        case .showNoResultsYet:
            commandHintBar.showStatus(LauncherStatusMessages.noResultsYet)
        case .activateSelected:
            activateSelectedItem()
        }
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

    func syncRowActionHint() {
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

    func usesColumnSplitLayout() -> Bool {
        currentHomeSplitState().columnSplitActive
    }

    private func currentHomeSplitState() -> LauncherHomeSplitState {
        LauncherHomeSplitPlanner.layout(
            queryTrimmedIsEmpty: searchBar.stringValue
                .trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
            showingDetail: contentCoordinator.showingDetail,
            showingResults: contentCoordinator.showingResults
        )
    }

    private func listHoldsKeyboardFocus() -> Bool {
        guard let responder = listView.window?.firstResponder else { return false }
        if responder === listView { return true }
        guard let view = responder as? NSView else { return false }
        return view.isDescendant(of: listView)
    }

    func syncSplitLayout() {
        let splitState = currentHomeSplitState()
        let columnSplit = splitState.columnSplitActive
        let rightPane = splitState.rightPane
        if columnSplit {
            LauncherInPanelLayout.ensureHomeSplitPanelSize(from: searchBar)
        }

        let layoutChanged = lastSplitLayoutState.map { $0 != (columnSplit, rightPane) } ?? true
        lastSplitLayoutState = (columnSplit, rightPane)

        homeSplitLayout.setColumnSplitActive(columnSplit)
        homeSplitLayout.setRightPane(rightPane)

        if columnSplit, rightPane == .guide {
            let commands = viewModel.commandRouter.registry.discoverableCommands
                .filter { cachedEnabledModuleIDs.contains($0.module) }
            let catalogIDs = commands.map(\.id)
            if catalogIDs != lastAppliedGuideCatalogIDs {
                homeSplitLayout.guidePane.applyCatalog(commands, enabledModules: cachedEnabledModuleIDs)
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
            performActionThenDismiss(action: action, for: item)
        default:
            performActionThenDismiss(action: action, for: item)
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
            resetSyncedQueryForRestore()
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
                ?? ModuleRegistry.defaultEnabledModuleIDs
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
                ?? ModuleRegistry.defaultEnabledModuleIDs
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
                ?? ModuleRegistry.defaultEnabledModuleIDs
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
            resetSyncedQueryForRestore()
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
            resetSyncedQueryForRestore()
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
                performActionThenDismiss(action: act, for: item)
            }
        case .todoDraft:
            searchBar.stringValue = TodoModule.resumeQuery(forCapture: result.preview)
            resetSyncedQueryForRestore()
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
        performActionThenDismiss(action: action, for: item)
    }

    private func performActionThenDismiss(action: Action, for item: ResultItem) {
        if let feedback = LauncherActionFeedback.feedback(for: action) {
            commandHintBar.showStatus(feedback.message)
        }
        Task {
            let result = await actionExecutor.run(action, for: item)
            await MainActor.run {
                guard result.succeeded else {
                    let message = result.userFacingMessage ?? LauncherStatusMessages.operationFailed
                    commandHintBar.showStatus(message)
                    return
                }
                if LauncherActionFeedback.shouldDelayDismiss(for: action) {
                    Task {
                        try? await Task.sleep(for: .milliseconds(900))
                        await MainActor.run { self.onActionDismiss() }
                    }
                } else {
                    onActionDismiss()
                }
            }
        }
    }

    private func runWithFeedback(action: Action, for item: ResultItem) {
        performActionThenDismiss(action: action, for: item)
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
        setSelectionIndex(index)
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
        let trimmed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let detailMode: LauncherContentMode = .detail(contentCoordinator.currentDetailModuleID)
        return LauncherKeyboardDispatcher.handle(command, context: .init(
            actionPanelVisible: actionPanel.isVisible,
            actionPanelActionCount: actionPanel.actionCount,
            showingDetail: contentCoordinator.showingDetail,
            listHoldsKeyboardFocus: listHoldsKeyboardFocus(),
            showingResults: contentCoordinator.showingResults,
            queryTrimmedIsEmpty: trimmed.isEmpty,
            itemCount: contentCoordinator.currentItems.count,
            selectedIndex: contentCoordinator.selectedIndex,
            currentItem: contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex],
            contentMode: detailMode,
            dismissActionPanel: { [weak self] in self?.actionPanel.dismiss() },
            openActionPanel: { [weak self] in self?.openActionPanel() },
            moveActionPanelSelection: { [weak self] delta in self?.actionPanel.moveSelection(delta: delta) },
            activateActionPanelIndex: { [weak self] index in self?.actionPanel.activateIndex(index) },
            moveListSelection: { [weak self] delta in
                self?.moveSelection(by: delta)
            },
            jumpToFlatIndex: { [weak self] index in
                guard let self else { return }
                self.setSelectionIndex(index)
                if let item = self.contentCoordinator.currentItems[safe: index] { self.handleRun(item) }
            },
            runItem: { [weak self] item in self?.run(item: item) },
            runSecondaryForSelected: { [weak self] in
                guard let self,
                      let item = self.contentCoordinator.currentItems[safe: self.contentCoordinator.selectedIndex],
                      let secondary = item.secondaryActions.first else { return false }
                self.runAction(secondary, for: item)
                return true
            },
            showNoAlternateActions: { [weak self] in
                self?.commandHintBar.showStatus(LauncherStatusMessages.noAlternateActions)
            }
        ))
    }
}

// MARK: - State write gates (Phase 11.2)

/// `LauncherContentCoordinator` remains the UI state store; these methods are the only
/// `LauncherRootController` entry points that mutate query routing, selection, content mode,
/// and panel-active snapshot apply policy. Direct coordinator/searchBar writes outside this
/// extension should be treated as regressions.
private extension LauncherRootController {

    func recordQueryTextChange(_ text: String, isEmpty: Bool) {
        lastSyncedQuery = text
        launcherEnvironment.isLauncherQueryEmpty = isEmpty
    }

    func resetSyncedQueryForRestore() {
        lastSyncedQuery = ""
    }

    func syncQueryBaselineFromSearchField() {
        lastSyncedQuery = searchBar.queryText
    }

    func recordQueryEmptyState() {
        launcherEnvironment.isLauncherQueryEmpty = true
    }

    func setSelectionIndex(_ index: Int) {
        contentCoordinator.updateSelection(to: index)
    }

    func moveSelection(by delta: Int) {
        let count = contentCoordinator.currentItems.count
        guard count > 0 else { return }
        let next = contentCoordinator.selectedIndex + delta
        setSelectionIndex(min(max(0, next), count - 1))
    }

    func transitionToEmptyQueryHome() {
        searchBar.cancelDetailMode()
        contentCoordinator.dismissResultsForEmptyQuery()
    }

    func transitionToResultsMode(clearStaleContent: Bool) {
        contentCoordinator.beginShowingResults(clearStaleContent: clearStaleContent)
    }

    func transitionToClearedResultsList() {
        listView.clear()
        contentCoordinator.beginShowingResults(clearStaleContent: true)
    }

    func markPanelInactiveForSnapshotApply() {
        isPanelActiveForQueryApply = false
        if sessionState.panel == .visible || sessionState.panel == .showing {
            applySessionEvent(.panelHideBegan)
        }
    }

    func markPanelActiveForSnapshotApply() {
        isPanelActiveForQueryApply = true
        applySessionEvent(.panelShowCompleted)
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
