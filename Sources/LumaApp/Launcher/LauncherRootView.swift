import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class LauncherRootView: NSView {
    private let glassBackground = NSVisualEffectView()
    private let searchBar = LumaSearchBar()
    private let sidebarContainer = NSView()
    private let sidebarHeader = NSTextField(labelWithString: "OPEN APPS")
    private let sidebarScrollView = NSScrollView()
    private let sidebarStack = FlippedStackView()
    private let sidebarSeparator = NSView()
    private let contentContainer = NSView()
    private let homeScrollView = NSScrollView()
    private let featureGridView = FeatureFlowView()
    private let resultsScrollView = NSScrollView()
    private let resultsStackView = FlippedStackView()
    private let loadingLabel = NSTextField(labelWithString: "Loading…")
    private let detailContainer = NSView()
    private let detailTopBar = NSView()
    private let detailTitleLabel = NSTextField(labelWithString: "")

    private let sortedCards: [FeatureCard]
    private let viewModel: LauncherViewModel
    private let actionExecutor: ActionExecutor
    private let onDismiss: () -> Void
    private let onActionDismiss: () -> Void
    private let onOpenSettings: () -> Void
    private let config: ConfigurationStore
    private let latencyHUD = LatencyHUDOverlayView()
    private var latencyHUDEnabled = false
    private var modulesReady = false
    private var suppressSessionPersistence = false

    private lazy var sidebarController = OpenAppsSidebarController(
        stack: sidebarStack,
        appActivationTracker: appActivationTracker,
        actionExecutor: actionExecutor,
        onActionDismiss: onActionDismiss
    )
    private let permissionController = PermissionBannerController()
    private let appActivationTracker: AppActivationTracker
    private lazy var contentCoordinator = LauncherContentCoordinator(
        homeScrollView: homeScrollView,
        resultsScrollView: resultsScrollView,
        detailContainer: detailContainer,
        detailTopBar: detailTopBar,
        detailTitleLabel: detailTitleLabel,
        contentContainer: contentContainer,
        resultsStackView: resultsStackView
    )
    private var featureGridController: LauncherFeatureGridController!

    init(
        cards: [FeatureCard],
        viewModel: LauncherViewModel,
        actionExecutor: ActionExecutor,
        appActivationTracker: AppActivationTracker,
        config: ConfigurationStore,
        onDismiss: @escaping () -> Void,
        onActionDismiss: @escaping () -> Void,
        onOpenSettings: @escaping () -> Void
    ) {
        self.config = config
        self.sortedCards = cards.sorted {
            if $0.position.row == $1.position.row {
                return $0.position.column < $1.position.column
            }
            return $0.position.row < $1.position.row
        }
        self.viewModel = viewModel
        self.actionExecutor = actionExecutor
        self.appActivationTracker = appActivationTracker
        self.onDismiss = onDismiss
        self.onActionDismiss = onActionDismiss
        self.onOpenSettings = onOpenSettings
        super.init(frame: .zero)

        featureGridController = LauncherFeatureGridController(
            featureGridView: featureGridView,
            sortedCards: sortedCards,
            onCardTapped: { [weak self] card in self?.openModuleDetail(for: card) }
        )

        contentCoordinator.onSessionChanged = { [weak self] in self?.saveCurrentSession() }
        contentCoordinator.onHomeSessionSaved = { [weak self] in self?.saveHomeSession() }

        searchBar.onEscape = { [weak self] in self?.handleEscape() }
        searchBar.onReturn = { [weak self] in self?.activateFirstSearchResult() }
        searchBar.onTextChange = { [weak self] text in self?.handleTextChange(text) }
        searchBar.onKeyCommand = { [weak self] command in self?.handleKeyCommand(command) ?? false }
        searchBar.onOpenSettings = onOpenSettings
        searchBar.onDetailKey = { [weak self] event in
            guard let self, self.contentCoordinator.showingDetail else { return false }
            guard self.searchBar.stringValue.isEmpty else { return false }
            return self.contentCoordinator.currentDetailObject?.handleKeyDown(event) ?? false
        }

        viewModel.onSnapshot = { [weak self] snapshot in
            self?.apply(snapshot: snapshot)
        }

        LauncherPanelChrome.install(on: self, glassBackground: glassBackground)
        LauncherLayoutBuilder.install(
            on: self,
            searchBar: searchBar,
            sidebarContainer: sidebarContainer,
            sidebarHeader: sidebarHeader,
            sidebarScrollView: sidebarScrollView,
            sidebarStack: sidebarStack,
            sidebarSeparator: sidebarSeparator,
            contentContainer: contentContainer,
            homeScrollView: homeScrollView,
            featureGridView: featureGridView,
            resultsScrollView: resultsScrollView,
            resultsStackView: resultsStackView,
            loadingLabel: loadingLabel,
            detailContainer: detailContainer,
            detailTopBar: detailTopBar,
            detailTitleLabel: detailTitleLabel,
            closeDetailTarget: self,
            closeDetailAction: #selector(closeDetail)
        )
        setupLatencyHUD()
        permissionController.install(in: self)
        showHome(persist: false)
        featureGridController.startSubscriptions()
        Task { await loadLatencyHUDPreference() }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func startFeatureGridSubscriptions() {
        featureGridController.startSubscriptions()
    }

    func stopFeatureGridSubscriptions() {
        featureGridController?.stopSubscriptions()
    }

    override func layout() {
        super.layout()
        LauncherPanelChrome.layoutSheen(on: self)
        featureGridView.frame.size.width = homeScrollView.contentView.bounds.width
        featureGridView.needsLayout = true
    }

    func setLatencyHUDEnabled(_ enabled: Bool) {
        latencyHUDEnabled = enabled
        latencyHUD.isHidden = !enabled
        if enabled { latencyHUD.refresh() }
    }

    func focusSearchField() {
        if contentCoordinator.showingDetail {
            contentCoordinator.currentDetailObject?.activate()
            return
        }
        searchBar.focus()
    }

    func setModulesReady(_ ready: Bool) {
        modulesReady = ready
        if ready, searchBar.stringValue.isEmpty {
            loadingLabel.isHidden = true
        } else if ready {
            handleTextChange(searchBar.stringValue)
        }
    }

    func showHome(focusSearch: Bool = true, persist: Bool = true) {
        searchBar.stringValue = ""
        ModuleDetailRegistry.isLauncherQueryEmpty = true
        contentCoordinator.tearDownDetailIfNeeded()
        contentCoordinator.resetResults()
        homeScrollView.isHidden = false
        homeScrollView.alphaValue = 1
        resultsScrollView.isHidden = true
        resultsScrollView.alphaValue = 0
        featureGridController.render()
        loadingLabel.isHidden = modulesReady
        refreshOpenApps()
        permissionController.refresh()
        if focusSearch { focusSearchField() }
        if persist { saveHomeSession() }
    }

    func refreshOpenApps() { Task { await sidebarController.refresh() } }
    func resetSidebarExpansion() { sidebarController.resetExpanded() }
    func refreshPermissionStatus() { permissionController.refresh() }
    func startPermissionPollingIfNeeded() { permissionController.startPollingIfNeeded() }
    func stopPermissionPolling() { permissionController.stopPolling() }

    func restoreLastSessionIfNeeded() {
        guard !contentCoordinator.showingDetail, searchBar.stringValue.isEmpty else { return }
        guard let config = ModuleDetailRegistry.config else { return }
        Task {
            let moduleRaw = await config.launcherLastModuleID()
            let query = await config.launcherLastQuery()
            let translateSource = await config.launcherTranslateSourceText()
            let translateOutput = await config.launcherTranslateOutputText()
            await MainActor.run {
                self.restoreLastSession(
                    moduleRaw: moduleRaw,
                    query: query,
                    translateSource: translateSource,
                    translateOutput: translateOutput
                )
            }
        }
    }

    func saveCurrentSession() {
        guard !suppressSessionPersistence, let config = ModuleDetailRegistry.config else { return }
        let moduleID = contentCoordinator.currentDetailModuleID
        let query = searchBar.stringValue
        let translateContent = (contentCoordinator.currentDetailObject as? TranslateDetailView)?.currentContent()
        Task {
            await config.setLauncherLastModuleID(moduleID?.rawValue)
            await config.setLauncherLastQuery(query)
            if let translateContent {
                await config.setLauncherTranslateSourceText(translateContent.source)
                await config.setLauncherTranslateOutputText(translateContent.output)
            }
        }
    }

    /// Clears transient search state after an action so the next hotkey opens a clean home screen.
    func resetForActionDismiss() {
        if contentCoordinator.showingDetail {
            saveCurrentSession()
            return
        }
        suppressSessionPersistence = true
        searchBar.stringValue = ""
        ModuleDetailRegistry.isLauncherQueryEmpty = true
        contentCoordinator.resetResults()
        viewModel.cancel()
        loadingLabel.isHidden = modulesReady
        suppressSessionPersistence = false
        saveHomeSession()
    }

    func openModuleDetail(for moduleID: ModuleIdentifier) {
        guard let detail = ModuleDetailRegistry.make(for: moduleID) else { return }
        contentCoordinator.present(detail, moduleID: moduleID)
    }

    func openModuleDetail(for card: FeatureCard, prefillTranslateText: String? = nil) {
        guard let detail = ModuleDetailRegistry.make(for: card.id) else {
            searchBar.stringValue = card.triggerKeyword
            searchBar.focus()
            handleTextChange(card.triggerKeyword)
            return
        }
        contentCoordinator.present(detail, moduleID: card.id, card: card, prefillTranslateText: prefillTranslateText)
    }

    func openTranslateDetail(with text: String) {
        guard let card = sortedCards.first(where: { $0.id == .translate }) else { return }
        contentCoordinator.pendingTranslateText = text
        if contentCoordinator.showingDetail,
           let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
            translate.prefill(text: text, autoTranslate: true)
            contentCoordinator.pendingTranslateText = nil
            return
        }
        openModuleDetail(for: card, prefillTranslateText: text)
    }

    func handleEscape() {
        if contentCoordinator.showingDetail {
            closeDetail()
            return
        }
        let trimmed = searchBar.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if contentCoordinator.showingResults || !trimmed.isEmpty {
            showHome(focusSearch: true, persist: true)
            return
        }
        onDismiss()
    }

    @objc func closeDetail() {
        contentCoordinator.closeDetail()
    }

    private func saveHomeSession() {
        guard !suppressSessionPersistence, let config = ModuleDetailRegistry.config else { return }
        Task {
            await config.setLauncherLastModuleID(nil)
            await config.setLauncherLastQuery(searchBar.stringValue)
        }
    }

    private func saveSearchQuery(_ query: String) {
        guard !suppressSessionPersistence, let config = ModuleDetailRegistry.config else { return }
        Task { await config.setLauncherLastQuery(query) }
    }

    private func restoreLastSession(moduleRaw: String?, query: String, translateSource: String, translateOutput: String) {
        suppressSessionPersistence = true
        defer { suppressSessionPersistence = false }
        if let moduleRaw {
            openModuleDetail(for: ModuleIdentifier(rawValue: moduleRaw))
            if ModuleIdentifier(rawValue: moduleRaw) == .translate,
               let translate = contentCoordinator.currentDetailObject as? TranslateDetailView {
                translate.restore(sourceText: translateSource, outputText: translateOutput)
            }
            return
        }
        showHome(focusSearch: false, persist: false)
        guard !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        searchBar.stringValue = query
        handleTextChange(query)
    }

    private func setupLatencyHUD() {
        latencyHUD.translatesAutoresizingMaskIntoConstraints = false
        latencyHUD.isHidden = true
        addSubview(latencyHUD)
        NSLayoutConstraint.activate([
            latencyHUD.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            latencyHUD.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -10)
        ])
    }

    private func loadLatencyHUDPreference() async {
        setLatencyHUDEnabled(await config.latencyHUDEnabled())
    }

    private func apply(snapshot: ResultSnapshot) {
        loadingLabel.isHidden = true
        contentCoordinator.apply(snapshot: snapshot) { [weak self] item in
            self?.run(item: item)
        }
        if let paintMs = LatencyTracker.shared.markFirstPaint() {
            LatencyTelemetry.report(p95Milliseconds: paintMs)
            if latencyHUDEnabled { latencyHUD.refresh() }
        }
    }

    func handleTextChange(_ text: String) {
        ModuleDetailRegistry.isLauncherQueryEmpty = text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        searchBar.setPlaceholder(ModuleSearchHints.placeholder(for: text))
        saveSearchQuery(text)
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            contentCoordinator.dismissResultsForEmptyQuery()
            viewModel.cancel()
            loadingLabel.isHidden = true
            return
        }
        if !modulesReady {
            loadingLabel.isHidden = false
            return
        }
        loadingLabel.isHidden = true
        contentCoordinator.beginShowingResults()
        viewModel.queryChanged(text, issuedAt: .now)
    }

    @objc private func activateFirstSearchResult() {
        guard contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex) else { return }
        run(item: contentCoordinator.currentItems[contentCoordinator.selectedIndex])
    }

    private func run(item: ResultItem) {
        if case .translateText(let text) = item.primaryAction.kind {
            openTranslateDetail(with: text)
            return
        }
        onActionDismiss()
        Task { await actionExecutor.run(item.primaryAction, for: item) }
    }

    private func handleKeyCommand(_ command: LumaSearchBar.KeyCommand) -> Bool {
        switch command {
        case .down:
            guard !contentCoordinator.currentItems.isEmpty else { return true }
            contentCoordinator.updateSelection(to: min(contentCoordinator.selectedIndex + 1, contentCoordinator.currentItems.count - 1))
            return true
        case .up:
            guard !contentCoordinator.currentItems.isEmpty else { return true }
            contentCoordinator.updateSelection(to: max(contentCoordinator.selectedIndex - 1, 0))
            return true
        case .tab:
            guard contentCoordinator.currentItems.indices.contains(contentCoordinator.selectedIndex),
                  let action = contentCoordinator.currentItems[contentCoordinator.selectedIndex].secondaryActions.first else { return true }
            let item = contentCoordinator.currentItems[contentCoordinator.selectedIndex]
            onActionDismiss()
            Task { await actionExecutor.run(action, for: item) }
            return true
        case .commandNumber(let number):
            if !contentCoordinator.showingResults {
                let index = number - 1
                featureGridController.updateHighlight(index: index)
                if sortedCards.indices.contains(index) {
                    openModuleDetail(for: sortedCards[index])
                }
                return true
            }
            let index = number - 1
            guard contentCoordinator.currentItems.indices.contains(index) else { return true }
            contentCoordinator.updateSelection(to: index)
            run(item: contentCoordinator.currentItems[index])
            return true
        }
    }
}
