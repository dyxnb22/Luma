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
    let permissionController = PermissionBannerController()

    private let performanceStrip: LauncherPerformanceStripView

    private let viewModel: LauncherViewModel
    private let homeCoordinator: LauncherHomeCoordinator
    private let actionExecutor: ActionExecutor
    private let config: ConfigurationStore
    private let sessionStore = LauncherSessionStore()
    private let launcherEnvironment: LauncherEnvironment
    private let onDismiss: () -> Void
    private let onActionDismiss: () -> Void
    private let onOpenSettings: () -> Void

    private var modulesReady = false
    private var homeRefreshTask: Task<Void, Never>?

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

    func setModulesReady(_ ready: Bool) {
        modulesReady = ready
        hintBar.setModulesReady(ready)
        if ready, searchBar.stringValue.isEmpty {
            refreshHome()
        } else if ready {
            handleTextChange(searchBar.stringValue)
        }
    }

    func showHome(focusSearch: Bool = true, persist: Bool = true) {
        searchBar.stringValue = ""
        commandHintBar.apply(nil)
        launcherEnvironment.isLauncherQueryEmpty = true
        contentCoordinator.tearDownDetailIfNeeded()
        contentCoordinator.resetResults()
        listView.isHidden = false
        listView.alphaValue = 1
        hintBar.setContext(.home)
        syncPerformanceStripVisibility()
        refreshHome()
        permissionController.refresh()
        if focusSearch { focusSearchField() }
        if persist { saveHomeSession() }
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
                self.hintBar.setContext(.home)
                self.syncRowActionHint()
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
        refreshHome()
    }

    func openModuleDetail(for moduleID: ModuleIdentifier, payload: Data? = nil) {
        if moduleID == .snippets, let payload,
           let action = try? ModuleActionCoding.decode(SnippetsAction.self, from: payload),
           case .create(let title) = action {
            Task {
                do {
                    _ = try await launcherEnvironment.snippetsModule.add(
                        title: title,
                        content: "",
                        tags: [],
                        trigger: ""
                    )
                    await MainActor.run {
                        self.launcherEnvironment.reloadSnippetsDetail()
                        self.presentModuleDetail(for: moduleID)
                    }
                } catch {
                    await MainActor.run {
                        self.commandHintBar.showStatus("Could not create snippet")
                    }
                }
            }
            return
        }
        applyModuleDetailPayload(moduleID: moduleID, payload: payload)
        presentModuleDetail(for: moduleID)
    }

    private func presentModuleDetail(for moduleID: ModuleIdentifier) {
        guard let detail = launcherEnvironment.makeDetailView(for: moduleID) else {
            showHome(focusSearch: true, persist: false)
            return
        }
        enterDetailContext()
        contentCoordinator.present(detail, moduleID: moduleID)
        syncRowActionHint()
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
        hintBar.setContext(.home)
        syncPerformanceStripVisibility()
    }

    func handleTextChange(_ text: String) {
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
        contentCoordinator.beginShowingResults()
        hintBar.setContext(.results)
        syncPerformanceStripVisibility()
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
        contentCoordinator.apply(snapshot: snapshot)
        hintBar.setContext(.results)
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
            commandHintBar.showStatus("Modules loading…")
            return
        }
        if actionPanel.isVisible {
            actionPanel.activateSelection()
            return
        }
        if !contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex) {
            if performBareCommandAction() { return }
            commandHintBar.showStatus("No results yet")
            return
        }
        activateSelectedItem()
    }

    @discardableResult
    private func performBareCommandAction() -> Bool {
        let raw = searchBar.stringValue
        let route = viewModel.commandRouter.route(raw: raw)
        guard case .targeted(let module, _, let payload) = route else { return false }
        guard let command = viewModel.commandRouter.registry.command(forModule: module),
              command.bareBehavior == .openDetail else { return false }

        var detailPayload: Data?
        if module == .wordbook,
           payload.compare("review", options: .caseInsensitive) == .orderedSame {
            detailPayload = try? ModuleActionCoding.encode(WordbookAction.review)
        }
        openModuleDetail(for: module, payload: detailPayload)
        return true
    }

    private func syncRowActionHint() {
        guard contentCoordinator.showingResults || !contentCoordinator.showingDetail else {
            commandHintBar.setReturnAction(nil)
            return
        }
        if let item = contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex] {
            commandHintBar.setReturnAction(item.returnHint)
        } else {
            commandHintBar.setReturnAction(nil)
        }
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
        default:
            onActionDismiss()
            Task { await actionExecutor.run(action, for: item) }
        }
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
        actionPanel.present(item: target, relativeTo: hintBar)
    }

    private func handleKeyCommand(_ command: LumaSearchBar.KeyCommand) -> Bool {
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
        case .commandNumber(let n): .commandNumber(n)
        }
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
