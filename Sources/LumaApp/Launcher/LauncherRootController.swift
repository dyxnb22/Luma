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

    private let performanceStrip: LauncherPerformanceStripView

    private let viewModel: LauncherViewModel
    private let homeCoordinator: LauncherHomeCoordinator
    private let actionExecutor: ActionExecutor
    private let config: ConfigurationStore
    private let sessionStore = LauncherSessionStore()
    private let launcherEnvironment: LauncherEnvironment
    private let workbenchCommandExecutor = WorkbenchCommandExecutor()
    private let onDismiss: () -> Void
    private let onActionDismiss: () -> Void
    private let onOpenSettings: () -> Void

    private var modulesReady = false
    private var homeRefreshTask: Task<Void, Never>?
    private var querySyncTimer: Timer?
    private var lastSyncedQuery = ""

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
        self.performanceStrip = performanceStrip
        self.launcherEnvironment = launcherEnvironment
        self.onDismiss = onDismiss
        self.onActionDismiss = onActionDismiss
        self.onOpenSettings = onOpenSettings
        self.permissionController = PermissionBannerController(config: config)

        wireCallbacks()
    }

    private func wireCallbacks() {
        contentCoordinator.onSessionChanged = { [weak self] in self?.saveCurrentSession() }
        contentCoordinator.onHomeSessionSaved = { [weak self] in self?.saveHomeSession() }
        contentCoordinator.onRun = { [weak self] item in self?.handleRun(item) }
        contentCoordinator.onRightClick = { [weak self] item in self?.openActionPanel(for: item) }
        contentCoordinator.onSelectionChanged = { [weak self] in self?.syncRowActionHint() }

        actionPanel.onRun = { [weak self] action, item in self?.runAction(action, for: item) }
        actionPanel.onClose = { [weak self] in self?.focusSearchField() }

        searchBar.onEscape = { [weak self] in self?.handleEscape() }
        searchBar.onReturn = { [weak self] in self?.activateReturn() }
        searchBar.onInterceptKeyDown = { [weak self] event in
            self?.actionPanel.handleKeyDown(event) ?? false
        }
        searchBar.onTextChange = { [weak self] text in self?.handleTextChange(text) }
        searchBar.onKeyCommand = { [weak self] command in self?.handleKeyCommand(command) ?? false }
        searchBar.onOpenSettings = onOpenSettings
        searchBar.onDetailKey = { [weak self] event in
            guard let self, self.contentCoordinator.showingDetail else { return false }
            guard self.searchBar.stringValue.isEmpty else { return false }
            return self.contentCoordinator.currentDetailObject?.handleKeyDown(event) ?? false
        }

        viewModel.onSnapshot = { [weak self] snapshot in self?.apply(snapshot: snapshot) }
    }

    private func previewWorkbenchCommand(route: WorkbenchCommandRoute, queryText: String) async {
        let enabled = await config.enabledModules()
            ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
        let pinned = await config.pinnedModuleIDs()
        let clipboard = await ClipboardPasteboardCache.shared.snapshot()
        let selection = await SelectionSnapshotService.shared.snapshot()
        let context = await WorkbenchContextBuilder().build(
            enabledModuleIDs: enabled,
            pinnedModuleIDs: pinned,
            clipboardPreview: clipboard,
            selectionText: selection
        )
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
                rowKind: .starter
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
        querySyncTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.syncQueryIfNeeded()
            }
        }
    }

    func stopQuerySync() {
        querySyncTimer?.invalidate()
        querySyncTimer = nil
    }

    private func syncQueryIfNeeded() {
        searchBar.commitEditingIfNeeded()
        let text = searchBar.queryText
        guard text != lastSyncedQuery else { return }
        if text.isEmpty {
            let committed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
            if !committed.isEmpty { return }
            if !lastSyncedQuery.isEmpty, searchBar.isActivelyEditing { return }
        }
        handleTextChange(text)
    }

    func showHome(focusSearch: Bool = true, persist: Bool = true) {
        homeRefreshTask?.cancel()
        viewModel.cancel()
        searchBar.resetQueryText()
        lastSyncedQuery = ""
        commandHintBar.apply(nil)
        launcherEnvironment.isLauncherQueryEmpty = true
        contentCoordinator.tearDownDetailIfNeeded()
        contentCoordinator.resetResults()
        listView.isHidden = false
        listView.alphaValue = 1
        syncKeyHints()
        syncPerformanceStripVisibility()
        refreshHome()
        permissionController.refresh()
        if focusSearch { focusSearchField() }
        if persist {
            saveHomeSession()
            persistResumeState(translateContent: nil)
        }
    }

    func refreshHome() {
        homeRefreshTask?.cancel()
        homeRefreshTask = Task {
            let snapshot = await homeCoordinator.snapshot()
            guard !Task.isCancelled else { return }
            await MainActor.run {
                guard self.searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                      !self.contentCoordinator.showingResults else { return }
                self.contentCoordinator.showHome(snapshot)
                self.syncRowActionHint()
                let suggestedKeys = snapshot.sections
                    .filter { $0.kind == .create || $0.kind == .continueFlow }
                    .flatMap(\.items)
                    .map(\.id.key)
                Task { await HomeSuggestionMemory.shared.recordShown(keys: suggestedKeys) }
                _ = HomeLatencyTracker.markHomeRendered()
            }
        }
    }

    func refreshOpenApps() { refreshHome() }

    func resetOpenAppsExpansion() {
        Task {
            await homeCoordinator.resetExpansion()
        }
    }

    func expandOpenApps() {
        Task {
            await homeCoordinator.expandAllApps()
            await MainActor.run { self.refreshHome() }
        }
    }

    func toggleOpenAppWindows(bundleID: String) {
        Task {
            await homeCoordinator.toggleAppWindows(bundleID: bundleID)
            await MainActor.run { self.refreshHome() }
        }
    }

    func focusSearchField() {
        if contentCoordinator.showingDetail {
            contentCoordinator.currentDetailObject?.activate()
            return
        }
        searchBar.focus()
    }

    func restoreLastSessionIfNeeded() {
        guard ProcessInfo.processInfo.environment["LUMA_QA"] != "1" else { return }
        guard !contentCoordinator.showingDetail, searchBar.stringValue.isEmpty else { return }
        Task {
            let persisted = await sessionStore.loadPersistedSession()
            await MainActor.run {
                self.applyRestore(
                    moduleRaw: persisted.moduleRaw,
                    query: persisted.query,
                    translateSource: persisted.translateSource,
                    translateOutput: persisted.translateOutput
                )
            }
        }
    }

    func saveCurrentSession() {
        let translateContent = (contentCoordinator.currentDetailObject as? TranslateDetailView)?.currentContent()
        sessionStore.saveCurrentSession(
            moduleID: contentCoordinator.currentDetailModuleID,
            query: searchBar.stringValue,
            translateContent: translateContent.map { ($0.source, $0.output) }
        )
        persistResumeState(translateContent: translateContent)
    }

    private func persistResumeState(translateContent: (source: String, output: String)?) {
        var query = searchBar.stringValue
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

        LauncherResumeStore.save(state)
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
                showStatus("Module disabled in Settings")
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
            guard await isModuleEnabledForDetail(moduleID) else {
                showStatus("Module disabled in Settings")
                return
            }
            await launcherEnvironment.warmModuleForDetail(moduleID)
            guard let detail = launcherEnvironment.makeDetailView(for: moduleID) else {
                showHome(focusSearch: true, persist: false)
                return
            }
            enterDetailContext()
            contentCoordinator.present(detail, moduleID: moduleID)
            if moduleID == .translate,
               let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
                let state = LauncherResumeStore.load()
                if !state.translateSource.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    translate.restore(sourceText: state.translateSource, outputText: state.translateOutput)
                }
            }
            syncRowActionHint()
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
        if actionPanel.isVisible {
            actionPanel.dismiss()
            return
        }
        if contentCoordinator.showingDetail {
            closeDetail()
            showHome(focusSearch: true, persist: true)
            return
        }
        let trimmed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if contentCoordinator.showingResults || !trimmed.isEmpty {
            showHome(focusSearch: true, persist: true)
            return
        }
        onDismiss()
    }

    func closeDetail() {
        contentCoordinator.closeDetail()
        Task { await launcherEnvironment.reserveDetailModule(nil) }
        syncKeyHints()
        syncPerformanceStripVisibility()
    }

    func showStatus(_ message: String) {
        commandHintBar.showStatus(message)
    }

    func handleTextChange(_ text: String) {
        guard text != lastSyncedQuery else { return }
        lastSyncedQuery = text
        launcherEnvironment.isLauncherQueryEmpty = text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        searchBar.setPlaceholder(ModuleSearchHints.placeholder(for: text))
        commandHintBar.apply(viewModel.commandRouter.registry.hint(for: text))
        sessionStore.saveSearchQuery(text)
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            contentCoordinator.dismissResultsForEmptyQuery()
            viewModel.cancel()
            refreshHome()
            return
        }
        guard modulesReady else { return }
        homeRefreshTask?.cancel()
        if contentCoordinator.showingDetail {
            closeDetail()
        }
        listView.isHidden = false
        listView.alphaValue = 1
        contentCoordinator.beginShowingResults(clearStaleContent: true)
        syncKeyHints()
        syncPerformanceStripVisibility()
        let workbenchRoute = viewModel.workbenchRoute(for: text)
        if workbenchRoute != .none {
            Task {
                await self.previewWorkbenchCommand(route: workbenchRoute, queryText: text)
            }
            return
        }
        viewModel.queryChanged(text, issuedAt: .now)
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
        case .openModule(let moduleID, let source, let output):
            openModuleDetail(for: moduleID)
            if moduleID == .translate,
               let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
                translate.restore(sourceText: source, outputText: output)
            }
        case .restoreQuery(let query):
            showHome(focusSearch: false, persist: false)
            searchBar.stringValue = query
            handleTextChange(query)
        case .showHome:
            showHome(focusSearch: false, persist: false)
        }
    }

    private func apply(snapshot: ResultSnapshot) {
        guard !launcherEnvironment.isLauncherQueryEmpty else { return }
        contentCoordinator.apply(snapshot: snapshot)
        syncPerformanceStripVisibility()
        syncRowActionHint()
        if let paintMs = LatencyTracker.shared.markFirstPaint() {
            LatencyTelemetry.report(p95Milliseconds: paintMs)
        }
    }

    private func syncPerformanceStripVisibility() {
        performanceStrip.setContentVisible(!contentCoordinator.showingDetail)
    }

    private func enterDetailContext() {
        hintBar.setContext(.detail)
        performanceStrip.setContentVisible(false)
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
                        launcherEnvironment.showStatus(LauncherStatusMessages.snippetExpanded)
                        try? await launcherEnvironment.snippetsModule.insertSnippet(id: snippet.id)
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

    private func activateSelectedItem() {
        guard contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex) else { return }
        handleRun(contentCoordinator.currentItems[contentCoordinator.selectedIndex])
    }

    private func handleRun(_ item: ResultItem) {
        switch LauncherKeyRouter.resolveRun(item: item) {
        case .expandOpenApps:
            expandOpenApps()
            return
        case .toggleOpenAppWindows(let bundleID):
            toggleOpenAppWindows(bundleID: bundleID)
            return
        case .runItem(let item):
            viewModel.recordExecutedCommand(for: searchBar.stringValue)
            dispatchAction(item.primaryAction, for: item)
        default:
            viewModel.recordExecutedCommand(for: searchBar.stringValue)
            dispatchAction(item.primaryAction, for: item)
        }
    }

    private func dispatchAction(_ action: Action, for item: ResultItem) {
        if item.id.key.hasPrefix("setup.") {
            Task { await config.setSetupHintsDismissed(true) }
        }
        switch action.kind {
        case .noop:
            focusSearchField()
        case .replaceQuery(let query):
            searchBar.stringValue = query
            handleTextChange(query)
            focusSearchField()
        case .openModuleDetail(let moduleID, let payload):
            recordHomeCompletionIfNeeded(for: item)
            openModuleDetail(for: moduleID, payload: payload)
        case .translateText(let text):
            recordHomeCompletionIfNeeded(for: item)
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
            rowKind: .starter
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
                rowKind: .starter
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
                rowKind: .starter
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
            recordHomeCompletionIfNeeded(for: item)
            runOpenLinkedEntity(linkID: linkID)
        case .openActivityEntry(let entryID):
            recordHomeCompletionIfNeeded(for: item)
            runOpenActivityEntry(entryID: entryID, for: item)
        case .showStatus(let message):
            commandHintBar.showStatus(message)
        }
    }

    func runOpenActivityEntry(entryID: UUID, for item: ResultItem?) {
        Task {
            let activities = await WorkbenchActivityStore.shared.allEntries()
            guard let entry = activities.first(where: { $0.id == entryID }) else {
                await MainActor.run { commandHintBar.showStatus("Activity no longer available") }
                return
            }
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            guard enabled.contains(entry.moduleID) else {
                await MainActor.run { commandHintBar.showStatus("Module disabled in Settings") }
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
                await MainActor.run { commandHintBar.showStatus("Linked item no longer available") }
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
                await MainActor.run { commandHintBar.showStatus("Module disabled in Settings") }
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
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            let pinned = await config.pinnedModuleIDs()
            let clipboard = await ClipboardPasteboardCache.shared.snapshot()
            let selection = await SelectionSnapshotService.shared.snapshot()
            let followUp = workbenchFollowUp(for: target)
            let runner = WorkbenchCaptureRunner()
            guard let result = await runner.runCapture(
                source: source,
                target: target,
                enabledModuleIDs: enabled,
                pinnedModuleIDs: pinned,
                clipboardPreview: clipboard,
                selectionText: selection,
                attribution: WorkbenchCaptureAttribution(sourceKind: .home, followUp: followUp)
            ) else {
                await MainActor.run {
                    if !enabled.contains(target.moduleID) {
                        commandHintBar.showStatus("Module disabled in Settings")
                    } else {
                        commandHintBar.showStatus("Nothing to capture")
                    }
                }
                return
            }
            await MainActor.run {
                recordHomeCompletionIfNeeded(for: item)
                applyWorkbenchCaptureResult(result, for: item)
                persistResumeState(translateContent: nil)
            }
        }
    }

    private func runWorkbenchResumeActivity(entryID: UUID, for item: ResultItem) {
        Task {
            let activities = await WorkbenchActivityStore.shared.allEntries()
            guard let entry = activities.first(where: { $0.id == entryID }) else {
                await MainActor.run { commandHintBar.showStatus("Activity no longer available") }
                return
            }
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            guard enabled.contains(entry.moduleID) else {
                await MainActor.run { commandHintBar.showStatus("Module disabled in Settings") }
                return
            }
            await MainActor.run {
                recordHomeCompletionIfNeeded(for: item)
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
            rowKind: .starter
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
            let enabled = await config.enabledModules()
                ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
            let pinned = await config.pinnedModuleIDs()
            let clipboard = await ClipboardPasteboardCache.shared.snapshot()
            let selection = await SelectionSnapshotService.shared.snapshot()
            let outcome = await workbenchCommandExecutor.handle(
                commandID: commandID,
                enabledModuleIDs: enabled,
                pinnedModuleIDs: pinned,
                clipboardPreview: clipboard,
                selectionText: selection
            )
            await MainActor.run {
                recordHomeCompletionIfNeeded(for: item)
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
            await recordHomeCompletion(for: item)
            try? await Task.sleep(for: .milliseconds(900))
            await MainActor.run { self.onActionDismiss() }
        }
    }

    private func runNotesOpen(path: String, action: Action, for item: ResultItem) {
        Task {
            if let daily = await launcherEnvironment.notesModule.dailyNotePath(), daily == path {
                await HomeSuggestionMemory.shared.recordDailyNoteOpened()
            }
            await recordHomeCompletion(for: item)
            await MainActor.run { self.onActionDismiss() }
            await actionExecutor.run(action, for: item)
        }
    }

    private func runWithFeedback(action: Action, for item: ResultItem) {
        if let feedback = LauncherActionFeedback.feedback(for: action) {
            commandHintBar.showStatus(feedback.message)
            Task {
                await actionExecutor.run(action, for: item)
                await recordHomeCompletion(for: item)
                await recordRecentAction(action: action, item: item)
                if feedback.delayDismiss {
                    try? await Task.sleep(for: .milliseconds(900))
                    await MainActor.run { self.onActionDismiss() }
                }
            }
        } else {
            onActionDismiss()
            Task {
                await actionExecutor.run(action, for: item)
                await recordHomeCompletion(for: item)
                await recordRecentAction(action: action, item: item)
            }
        }
    }

    private func recordHomeCompletionIfNeeded(for item: ResultItem) {
        guard item.rowKind == .starter else { return }
        Task { await recordHomeCompletion(for: item) }
    }

    private func recordHomeCompletion(for item: ResultItem) async {
        if item.id.key.hasPrefix("contextual.") {
            await HomeSuggestionMemory.shared.recordCompleted(key: item.id.key)
        }
        if item.id.key.hasPrefix("setup.") {
            await config.setSetupHintsDismissed(true)
        }
    }

    private func saveModuleRoundTripResume(module: ModuleIdentifier, query: String? = nil) {
        let resolved = LauncherModuleResumeQuery.normalizedQuery(
            for: module,
            raw: query ?? searchBar.stringValue
        )
        LauncherResumeStore.save(LauncherResumeState(moduleRaw: module.rawValue, query: resolved))
    }

    private func recordRecentAction(action: Action, item: ResultItem) async {
        await RecentActionMemory.shared.record(action: action, item: item)
    }

    private func run(item: ResultItem) {
        dispatchAction(item.primaryAction, for: item)
    }

    private func runAction(_ action: Action, for item: ResultItem) {
        if action.id == item.primaryAction.id {
            handleRun(item)
            return
        }
        dispatchAction(action, for: item)
    }

    private func openTranslateDetail(with text: String) {
        contentCoordinator.pendingTranslateText = text
        if contentCoordinator.showingDetail,
           let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
            translate.prefill(text: text, autoTranslate: true)
            contentCoordinator.pendingTranslateText = nil
            return
        }
        enterDetailContext()
        guard let detail = launcherEnvironment.makeDetailView(for: .translate) else { return }
        contentCoordinator.present(detail, moduleID: .translate, prefillTranslateText: text)
    }

    private func openActionPanel(for item: ResultItem? = nil) {
        let target = item ?? contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex]
        guard let target else { return }
        guard !target.secondaryActions.isEmpty else {
            commandHintBar.showStatus(LauncherStatusMessages.noActionPanelActions)
            return
        }
        actionPanel.present(item: target, relativeTo: hintBar)
    }

    private func handleKeyCommand(_ command: LumaSearchBar.KeyCommand) -> Bool {
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
        let mode: LauncherContentMode = contentCoordinator.showingDetail ? .detail
            : (contentCoordinator.showingResults ? .results : .home)
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
        case .expandOpenApps:
            expandOpenApps()
            return true
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
