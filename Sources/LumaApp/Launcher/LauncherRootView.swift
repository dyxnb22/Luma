import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

@MainActor
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
    private let detailContainer = NSView()
    private let detailTopBar = NSView()
    private let detailTitleLabel = NSTextField(labelWithString: "")
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
            closeDetailAction: #selector(closeDetailAction)
        )
        installLatencyHUD()
        controller.permissionController.install(in: self)
        controller.showHome(persist: false)
        Task { setLatencyHUDEnabled(await config.latencyHUDEnabled()) }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func setLatencyHUDEnabled(_ enabled: Bool) {
        latencyHUDEnabled = enabled
        latencyHUD.isHidden = !enabled
        if enabled { latencyHUD.refresh() }
    }

    func setModulesReady(_ ready: Bool) { controller.setModulesReady(ready) }
    func showHome(focusSearch: Bool = true, persist: Bool = true) { controller.showHome(focusSearch: focusSearch, persist: persist) }
    func setHomeProvidersActive(_ active: Bool) {
        Task { [homeCoordinator] in await homeCoordinator.setActive(active) }
    }
    func refreshOpenApps() { controller.refreshOpenApps() }
    func resetOpenAppsExpansion() { controller.resetOpenAppsExpansion() }
    func focusSearchField() { controller.focusSearchField() }
    func restoreLastSessionIfNeeded() { controller.restoreLastSessionIfNeeded() }
    func saveCurrentSession() { controller.saveCurrentSession() }
    func resetForActionDismiss() { controller.resetForActionDismiss() }
    func openModuleDetail(for moduleID: ModuleIdentifier) { controller.openModuleDetail(for: moduleID) }
    func runWorkbenchCaptureFromDetail(source: WorkbenchCaptureSource, target: WorkbenchCaptureTarget) {
        controller.runWorkbenchCaptureFromDetail(source: source, target: target)
    }
    func runWorkspaceRowActionFromDetail(_ action: CurrentProjectWorkspaceRowAction) {
        controller.runWorkspaceRowActionFromDetail(action)
    }
    func handleEscape() { controller.handleEscape() }
    func closeDetail() { controller.closeDetail() }

    func showStatus(_ message: String) { controller.showStatus(message) }

    @objc private func closeDetailAction() { controller.closeDetail() }
    func prepareDetailForHide() async { await contentCoordinator.currentDetailObject?.prepareForLauncherHide() }
    func refreshPermissionStatus() { controller.permissionController.refresh() }
    func startPermissionPollingIfNeeded() { controller.permissionController.startPollingIfNeeded() }
    func stopPermissionPolling() { controller.permissionController.stopPolling() }

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
