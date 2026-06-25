import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class LauncherRootController {
    let searchBar: LumaSearchBar
    let listView: LauncherListView
    let hintBar: LauncherHintBar
    let actionPanel: LauncherActionPanel
    let contentCoordinator: LauncherContentCoordinator
    let permissionController = PermissionBannerController()

    private let viewModel: LauncherViewModel
    private let homeCoordinator: LauncherHomeCoordinator
    private let actionExecutor: ActionExecutor
    private let config: ConfigurationStore
    private let sessionStore = LauncherSessionStore()
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
        listView: LauncherListView,
        hintBar: LauncherHintBar,
        actionPanel: LauncherActionPanel,
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
        self.listView = listView
        self.hintBar = hintBar
        self.actionPanel = actionPanel
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
        ModuleDetailRegistry.isLauncherQueryEmpty = true
        contentCoordinator.tearDownDetailIfNeeded()
        contentCoordinator.resetResults()
        listView.isHidden = false
        listView.alphaValue = 1
        hintBar.setContext(.home)
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
                _ = HomeLatencyTracker.markHomeRendered()
            }
        }
    }

    func refreshOpenApps() { refreshHome() }

    func resetOpenAppsExpansion() {
        Task { await homeCoordinator.resetOpenAppsExpansion(); await MainActor.run { self.refreshHome() } }
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
        ModuleDetailRegistry.isLauncherQueryEmpty = true
        contentCoordinator.resetResults()
        viewModel.cancel()
        sessionStore.suppressPersistence = false
        saveHomeSession()
        refreshHome()
    }

    func openModuleDetail(for moduleID: ModuleIdentifier) {
        guard let detail = ModuleDetailRegistry.make(for: moduleID) else {
            showHome(focusSearch: true, persist: false)
            return
        }
        hintBar.setContext(.detail)
        contentCoordinator.present(detail, moduleID: moduleID)
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
    }

    func handleTextChange(_ text: String) {
        ModuleDetailRegistry.isLauncherQueryEmpty = text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        searchBar.setPlaceholder(ModuleSearchHints.placeholder(for: text))
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
        if let paintMs = LatencyTracker.shared.markFirstPaint() {
            LatencyTelemetry.report(p95Milliseconds: paintMs)
        }
    }

    private func activateReturn() {
        if actionPanel.isVisible {
            actionPanel.activateSelection()
            return
        }
        activateSelectedItem()
    }

    private func activateSelectedItem() {
        guard contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex) else { return }
        handleRun(contentCoordinator.currentItems[contentCoordinator.selectedIndex])
    }

    private func handleRun(_ item: ResultItem) {
        switch LauncherKeyRouter.resolveRun(item: item) {
        case .expandOpenApps:
            Task { await homeCoordinator.expandOpenApps(); await MainActor.run { self.refreshHome() } }
        case .openTodoDetail:
            openModuleDetail(for: .todo)
        case .openClipboardDetail:
            openModuleDetail(for: .clipboard)
        case .openRecordsDetail:
            openModuleDetail(for: .media)
        case .runItem(let item):
            run(item: item)
        default:
            run(item: item)
        }
    }

    private func run(item: ResultItem) {
        if case .translateText(let text) = item.primaryAction.kind {
            openTranslateDetail(with: text)
            return
        }
        onActionDismiss()
        Task { await actionExecutor.run(item.primaryAction, for: item) }
    }

    private func runAction(_ action: Action, for item: ResultItem) {
        if action.id == item.primaryAction.id {
            handleRun(item)
            return
        }
        if case .translateText(let text) = action.kind {
            openTranslateDetail(with: text)
            return
        }
        onActionDismiss()
        Task { await actionExecutor.run(action, for: item) }
    }

    private func openTranslateDetail(with text: String) {
        contentCoordinator.pendingTranslateText = text
        if contentCoordinator.showingDetail,
           let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
            translate.prefill(text: text, autoTranslate: true)
            contentCoordinator.pendingTranslateText = nil
            return
        }
        hintBar.setContext(.detail)
        guard let detail = ModuleDetailRegistry.make(for: .translate) else { return }
        contentCoordinator.present(detail, moduleID: .translate, prefillTranslateText: text)
    }

    private func openActionPanel(for item: ResultItem? = nil) {
        let target = item ?? contentCoordinator.currentItems[safe: contentCoordinator.selectedIndex]
        guard let target else { return }
        actionPanel.present(item: target, relativeTo: hintBar)
    }

    private func handleKeyCommand(_ command: LumaSearchBar.KeyCommand) -> Bool {
        if actionPanel.isVisible {
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
        case .expandOpenApps, .openTodoDetail, .openClipboardDetail, .openRecordsDetail:
            handleRun(contentCoordinator.currentItems[contentCoordinator.selectedIndex])
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
