import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class LauncherWindowController {
    private let panel = LauncherPanel()
    private let panelContentHost = NSView()
    private var rootView: LauncherRootView?
    private var onWillShow: (() -> Void)?
    private var onDidHide: (() -> Void)?
    private var deferredShowWorkItem: DispatchWorkItem?
    private var showGeneration: UInt = 0
    private var lastPositionedVisibleFrame: CGRect?
    private var panelShown = false
    private var lastToggleAt: ContinuousClock.Instant?

    var isPanelVisible: Bool { panelShown }

    init() {
        panel.onEscape = { [weak self] in
            guard let self else { return }
            if let rootView = self.rootView {
                rootView.handleEscape()
            } else {
                self.hide()
            }
        }
        panel.onToggleHotkey = { [weak self] in
            self?.toggle()
        }
        panel.orderOut(nil)
    }

    func configure(
        viewModel: LauncherViewModel,
        homeCoordinator: LauncherHomeCoordinator,
        actionExecutor: ActionExecutor,
        config: ConfigurationStore,
        launcherEnvironment: LauncherEnvironment,
        onWillShow: @escaping () -> Void = {},
        onDidHide: @escaping () -> Void = {},
        onOpenSettings: @escaping () -> Void
    ) {
        self.onWillShow = onWillShow
        self.onDidHide = onDidHide
        let rootView = LauncherRootView(
            viewModel: viewModel,
            homeCoordinator: homeCoordinator,
            actionExecutor: actionExecutor,
            config: config,
            launcherEnvironment: launcherEnvironment,
            onDismiss: { [weak self] in self?.hide() },
            onActionDismiss: { [weak self] in self?.hideImmediatelyForAction() },
            onOpenSettings: onOpenSettings
        )
        panelContentHost.translatesAutoresizingMaskIntoConstraints = true
        panelContentHost.autoresizingMask = [.width, .height]
        panel.contentView = panelContentHost

        rootView.translatesAutoresizingMaskIntoConstraints = false
        panelContentHost.addSubview(rootView)
        NSLayoutConstraint.activate([
            rootView.leadingAnchor.constraint(equalTo: panelContentHost.leadingAnchor),
            rootView.trailingAnchor.constraint(equalTo: panelContentHost.trailingAnchor),
            rootView.topAnchor.constraint(equalTo: panelContentHost.topAnchor),
            rootView.bottomAnchor.constraint(equalTo: panelContentHost.bottomAnchor)
        ])
        self.rootView = rootView
        panel.onDetailKeyDown = { [weak rootView] event in
            rootView?.dispatchDetailKeyDown(event) ?? false
        }
        panel.onCloseDetail = { [weak rootView] in
            rootView?.dispatchDetailCloseFromKeyboard() ?? false
        }
        AppleTranslationHost.shared.attach(to: rootView)
    }

    func refreshHome() {
        rootView?.refreshHome()
    }

    func invalidatePanelSignalsCache() {
        rootView?.invalidatePanelSignalsCache()
    }

    func invalidatePermissionModuleCache() {
        rootView?.invalidatePermissionModuleCache()
    }

    func refreshHomeForBackgroundDataUpdate() {
        guard !panel.isVisible else { return }
        rootView?.refreshHome()
    }

    func setLatencyHUDEnabled(_ enabled: Bool) {
        rootView?.setLatencyHUDEnabled(enabled)
    }

    func setModulesReady(_ ready: Bool) {
        rootView?.setModulesReady(ready)
    }

    func closeDetailIfShowing() {
        rootView?.exitDetailFromChrome()
    }

    func exitDetailFromChromeIfShowing() {
        rootView?.exitDetailFromChrome()
    }

    func showStatus(_ message: String) {
        rootView?.showStatus(message)
    }

    func openModuleDetail(for moduleID: ModuleIdentifier) {
        rootView?.openModuleDetail(for: moduleID)
    }

    func runWorkbenchCaptureFromDetail(source: WorkbenchCaptureSource, target: WorkbenchCaptureTarget) {
        rootView?.runWorkbenchCaptureFromDetail(source: source, target: target)
    }

    func runWorkspaceRowActionFromDetail(_ action: CurrentProjectWorkspaceRowAction) {
        rootView?.runWorkspaceRowActionFromDetail(action)
    }

    func toggle() {
        let now = ContinuousClock.now
        if let lastToggleAt, now - lastToggleAt < .milliseconds(120) {
            return
        }
        lastToggleAt = now
        panelShown ? hide() : show()
    }

    func show() {
        cancelDeferredShowWork()
        showGeneration &+= 1
        let generation = showGeneration
        panelShown = true

        onWillShow?()
        HomeLatencyTracker.markHotkey()
        positionPanel()
        let previousBundleID = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        LauncherMenuTarget.set(bundleID: previousBundleID)
        Task {
            await MenuBarTreeService.shared.setLauncherContextBundleID(previousBundleID)
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else { return }
            await MenuBarTreeService.shared.scheduleRefreshForFrontmost()
        }

        NSApp.activate(ignoringOtherApps: true)
        panel.orderFrontRegardless()
        panel.makeKey()
        panel.alphaValue = 1

        DispatchQueue.main.async { [weak self] in
            guard let self, self.showGeneration == generation, self.panelShown else { return }
            self.rootView?.focusSearchFieldAfterShow()
            if ProcessInfo.processInfo.environment["LUMA_QA"] == "1" {
                self.ensureSearchFieldFocused()
            }
        }

        // Heavier panel services stay off the hotkey→visible path.
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.showGeneration == generation, self.panelShown else { return }
            self.rootView?.refreshPermissionStatus()
            self.rootView?.startPermissionPollingIfNeeded()
            self.rootView?.startPerformanceSampling()
            self.rootView?.setPanelSignalsActive(true)
            self.rootView?.restoreLastSessionIfNeeded()
        }
        deferredShowWorkItem = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.075, execute: work)
    }

    private func cancelDeferredShowWork() {
        deferredShowWorkItem?.cancel()
        deferredShowWorkItem = nil
    }

    func hide() {
        guard panelShown else { return }
        cancelDeferredShowWork()
        showGeneration &+= 1
        let generationAtHide = showGeneration
        panelShown = false
        rootView?.cancelPendingRestore()
        panel.alphaValue = 0
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            self.finishHide(generationAtHide: generationAtHide)
        }
    }

    private func clearMenuTarget() {
        LauncherMenuTarget.clear()
        Task {
            await MenuBarTreeService.shared.setLauncherContextBundleID(nil)
        }
    }

    private func finishHide(generationAtHide: UInt) {
        guard showGeneration == generationAtHide else { return }
        clearMenuTarget()
        rootView?.invalidatePanelSignalsCache()
        rootView?.flushPendingSessionWrites()
        rootView?.saveCurrentSession()
        rootView?.resetHomeExpansion()
        rootView?.stopPermissionPolling()
        rootView?.stopPerformanceSampling()
        rootView?.setPanelSignalsActive(false)
        panel.orderOut(nil)
        panel.alphaValue = 1
        onDidHide?()
    }

    func hideImmediatelyForAction() {
        guard panelShown else { return }
        cancelDeferredShowWork()
        showGeneration &+= 1
        panelShown = false
        rootView?.cancelPendingRestore()
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            self.clearMenuTarget()
            self.rootView?.invalidatePanelSignalsCache()
            self.rootView?.flushPendingSessionWrites()
            self.rootView?.resetForActionDismiss()
            self.rootView?.resetHomeExpansion()
            self.rootView?.stopPermissionPolling()
            self.rootView?.stopPerformanceSampling()
            self.rootView?.setPanelSignalsActive(false)
            self.panel.orderOut(nil)
            self.panel.alphaValue = 1
            self.onDidHide?()
        }
    }

    func hideIfShowingForExternalActivation(bundleID: String?) {
        guard panelShown else { return }
        guard bundleID != Bundle.main.bundleIdentifier else { return }
        hideImmediatelyForAction()
    }

    private func ensureSearchFieldFocused() {
        NSApp.activate(ignoringOtherApps: true)
        panel.makeKey()
        rootView?.focusSearchField()
    }

    private func positionPanel() {
        guard let screen = LumaPresentationScreen.current() else { return }
        let visibleFrame = screen.visibleFrame
        panel.position(on: visibleFrame)
        lastPositionedVisibleFrame = visibleFrame
    }

}
