@preconcurrency import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

// AppKit display cycle calls layout() without Swift MainActor executor — do not isolate this view.
final class LauncherRootView: NSView {
    private let glassBackground = NSVisualEffectView()
    private let performanceStrip = LauncherPerformanceStripView()
    private let resourceSampler = SystemResourceSampler()
    private let searchBar = LumaSearchBar()
    private let commandHintBar = CommandHintBar()
    private let listView = LauncherListView()
    private let hintBar = LauncherHintBar()
    private let actionPanel = LauncherActionPanel()
    private let contentContainer = NSView()
    private let detailContainer = LauncherOverlayHostView()
    private let detailTopBar = NSView()
    private let detailTitleLabel = NSTextField(labelWithString: "")
    private lazy var homeSplitLayout = LauncherHomeSplitLayout.install(
        in: contentContainer,
        listView: listView,
        detailContainer: detailContainer
    )
    private let latencyHUD = LatencyHUDOverlayView()
    private var latencyHUDEnabled = false

    private lazy var contentCoordinator = LauncherContentCoordinator(
        listView: listView,
        detailContainer: detailContainer,
        detailTopBar: detailTopBar,
        detailTitleLabel: detailTitleLabel,
        contentContainer: contentContainer
    )

    private lazy var controller = LauncherRootController(
        viewModel: viewModel,
        homeCoordinator: homeCoordinator,
        actionExecutor: actionExecutor,
        config: config,
        contentCoordinator: contentCoordinator,
        searchBar: searchBar,
        commandHintBar: commandHintBar,
        listView: listView,
        hintBar: hintBar,
        actionPanel: actionPanel,
        homeSplitLayout: homeSplitLayout,
        performanceStrip: performanceStrip,
        launcherEnvironment: launcherEnvironment,
        onDismiss: onDismiss,
        onActionDismiss: onActionDismiss,
        onOpenSettings: onOpenSettings
    )

    private let viewModel: LauncherViewModel
    private let homeCoordinator: LauncherHomeCoordinator
    private let actionExecutor: ActionExecutor
    private let config: ConfigurationStore
    private let launcherEnvironment: LauncherEnvironment
    private let onDismiss: () -> Void
    private let onActionDismiss: () -> Void
    private let onOpenSettings: () -> Void

    init(
        viewModel: LauncherViewModel,
        homeCoordinator: LauncherHomeCoordinator,
        actionExecutor: ActionExecutor,
        config: ConfigurationStore,
        launcherEnvironment: LauncherEnvironment,
        onDismiss: @escaping () -> Void,
        onActionDismiss: @escaping () -> Void,
        onOpenSettings: @escaping () -> Void
    ) {
        self.config = config
        self.launcherEnvironment = launcherEnvironment
        self.viewModel = viewModel
        self.homeCoordinator = homeCoordinator
        self.actionExecutor = actionExecutor
        self.onDismiss = onDismiss
        self.onActionDismiss = onActionDismiss
        self.onOpenSettings = onOpenSettings
        super.init(frame: .zero)

        LauncherPanelChrome.install(on: self, glassBackground: glassBackground)
        setContentHuggingPriority(.defaultLow, for: .horizontal)
        setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        LauncherLayoutBuilder.install(
            on: self,
            performanceStrip: performanceStrip,
            searchBar: searchBar,
            commandHintBar: commandHintBar,
            listView: listView,
            hintBar: hintBar,
            actionPanel: actionPanel,
            contentContainer: contentContainer,
            detailContainer: detailContainer,
            detailTopBar: detailTopBar,
            detailTitleLabel: detailTitleLabel,
            closeDetailTarget: self,
            closeDetailAction: #selector(closeDetailAction),
            onPanelSpacingChanged: { [weak self] in
                guard let self else { return }
                LauncherInPanelLayout.stabilizePanel(from: self.searchBar)
            }
        )
        _ = homeSplitLayout
        installLatencyHUD()
        controller.permissionController.install(in: self, above: hintBar)
        controller.showHome(persist: false)
        Task { setLatencyHUDEnabled(await config.latencyHUDEnabled()) }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    nonisolated override func layout() {
        super.layout()
        LauncherPanelChrome.layoutChromeLayers(on: self)
    }

    @MainActor
    func setLatencyHUDEnabled(_ enabled: Bool) {
        latencyHUDEnabled = enabled
        latencyHUD.isHidden = !enabled
        if enabled { latencyHUD.refresh() }
    }

    @MainActor
    func setModulesReady(_ ready: Bool) { controller.setModulesReady(ready) }
    @MainActor
    func showHome(focusSearch: Bool = true, persist: Bool = true) { controller.showHome(focusSearch: focusSearch, persist: persist) }
    @MainActor
    func setPanelSignalsActive(_ active: Bool) {
        Task { [homeCoordinator] in await homeCoordinator.setActive(active) }
    }
    @MainActor
    func refreshHome(intent: LauncherHomeRefreshIntent = .visibleRepaint) {
        controller.refreshHome(intent: intent)
    }
    @MainActor
    func resetHomeExpansion() { controller.resetHomeExpansion() }
    @MainActor
    func focusSearchField() { controller.focusSearchField() }
    @MainActor
    func focusSearchFieldAfterShow() { controller.focusSearchFieldAfterShow() }
    @MainActor
    func dispatchDetailKeyDown(_ event: NSEvent) -> Bool { controller.dispatchDetailKeyDown(event) }
    @MainActor
    func dispatchDetailCloseFromKeyboard() -> Bool { controller.dispatchDetailCloseFromKeyboard() }
    @MainActor
    var isShowingDetail: Bool { controller.isShowingDetail }
    @MainActor
    func cancelPendingRestore() { controller.cancelPendingRestore() }
    @MainActor
    func cancelActiveQueryAndSnapshotApply() { controller.cancelActiveQueryAndSnapshotApply() }
    @MainActor
    func cancelLauncherAsyncWork() { controller.cancelLauncherAsyncWork() }
    @MainActor
    func handleModulesDisabled(removed: Set<ModuleIdentifier>) { controller.handleModulesDisabled(removed: removed) }
    @MainActor
    func restoreLastSessionIfNeeded() { controller.restoreLastSessionIfNeeded() }
    @MainActor
    func saveCurrentSession() { controller.saveCurrentSession() }
    @MainActor
    func resetForActionDismiss() { controller.resetForActionDismiss() }
    @MainActor
    func openModuleDetail(for moduleID: ModuleIdentifier) { controller.openModuleDetail(for: moduleID) }
    @MainActor
    func performQABareCommandAction(raw: String) -> Bool { controller.performQABareCommandAction(raw: raw) }
    @MainActor
    func runWorkbenchCaptureFromDetail(source: WorkbenchCaptureSource, target: WorkbenchCaptureTarget) {
        controller.runWorkbenchCaptureFromDetail(source: source, target: target)
    }
    @MainActor
    func runWorkspaceRowActionFromDetail(_ action: CurrentProjectWorkspaceRowAction) {
        controller.runWorkspaceRowActionFromDetail(action)
    }
    @MainActor
    func handleEscape() { controller.handleEscape() }
    @MainActor
    func closeDetail() { controller.closeDetail() }
    @MainActor
    func exitDetailFromChrome() { controller.exitDetailFromChrome() }

    @MainActor
    func showStatus(_ message: String) { controller.showStatus(message) }

    @MainActor
    @objc private func closeDetailAction() { controller.exitDetailFromChrome() }
    @MainActor
    func prepareDetailForHide() async { await contentCoordinator.currentDetailObject?.prepareForLauncherHide() }
    @MainActor
    func flushPendingSessionWrites() { controller.flushPendingSessionWrites() }

    @MainActor
    func configurePanelSnapshotContext(
        _ provider: @escaping () -> (visibilitySessionVisible: Bool, visibilityGeneration: UInt, panelVisible: Bool)
    ) {
        controller.panelSnapshotContext = provider
    }

    @MainActor
    func exportStateSnapshot(reason: String? = nil) {
        controller.exportStateSnapshot(reason: reason)
    }

    @MainActor
    func reconcileLauncherStateAfterShow() {
        controller.reconcileLauncherStateAfterShow()
    }

    @MainActor
    func invalidatePanelSignalsCache() { controller.invalidatePanelSignalsCache() }

    @MainActor
    func invalidatePermissionModuleCache() { controller.invalidatePermissionModuleCache() }

    @MainActor
    func refreshPermissionStatus() { controller.refreshPermissionBanner() }
    @MainActor
    func startPermissionPollingIfNeeded() { controller.permissionController.startPollingIfNeeded() }
    @MainActor
    func stopPermissionPolling() { controller.permissionController.stopPolling() }

    @MainActor
    func startPerformanceSampling() {
        resourceSampler.onUpdate = { [weak self] presentation in
            self?.performanceStrip.apply(presentation)
        }
        resourceSampler.summaryProvider = { [launcherEnvironment] in
            async let todayCount: Int? = {
                return try? await launcherEnvironment.todoModule.todayDueCount()
            }()
            async let reviewCount: Int? = {
                return try? await launcherEnvironment.wordbookStore.dueTodayCount()
            }()
            return await PerformanceStripSummarySnapshot(
                todayCount: todayCount,
                reviewCount: reviewCount
            )
        }
        resourceSampler.start()
        controller.startQuerySync()
    }

    @MainActor
    func stopPerformanceSampling() {
        controller.stopQuerySync()
        resourceSampler.stop()
        resourceSampler.onUpdate = nil
        resourceSampler.summaryProvider = nil
    }

    private func installLatencyHUD() {
        latencyHUD.translatesAutoresizingMaskIntoConstraints = false
        latencyHUD.isHidden = true
        addSubview(latencyHUD)
        NSLayoutConstraint.activate([
            latencyHUD.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            latencyHUD.bottomAnchor.constraint(equalTo: hintBar.topAnchor, constant: -4)
        ])
    }
}
