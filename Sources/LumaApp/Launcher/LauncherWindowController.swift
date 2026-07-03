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

    init() {
        panel.onEscape = { [weak self] in
            guard let self else { return }
            if let rootView = self.rootView {
                rootView.handleEscape()
            } else {
                self.hide()
            }
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
        panel.isVisible ? hide() : show()
    }

    func show() {
        onWillShow?()
        HomeLatencyTracker.markHotkey()
        positionPanel()
        let previousBundleID = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        LauncherMenuTarget.set(bundleID: previousBundleID)
        Task {
            await MenuBarTreeService.shared.setLauncherContextBundleID(previousBundleID)
            await MenuBarTreeService.shared.scheduleRefreshForFrontmost()
        }
        panel.alphaValue = 0
        NSApp.activate(ignoringOtherApps: true)
        panel.orderFrontRegardless()
        panel.makeKey()
        rootView?.refreshPermissionStatus()
        rootView?.startPermissionPollingIfNeeded()
        rootView?.startPerformanceSampling()
        rootView?.setHomeProvidersActive(true)
        rootView?.restoreLastSessionIfNeeded()
        rootView?.focusSearchFieldAfterShow()
        rootView?.refreshHome()
        let duration = MotionTokens.panelShowDuration
        let qaMode = ProcessInfo.processInfo.environment["LUMA_QA"] == "1"
        NSAnimationContext.runAnimationGroup { context in
            context.duration = duration
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            panel.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                if qaMode {
                    self.ensureSearchFieldFocused()
                }
            }
        }
    }

    func hide() {
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            self.finishHide()
        }
    }

    private func clearMenuTarget() {
        LauncherMenuTarget.clear()
        Task {
            await MenuBarTreeService.shared.setLauncherContextBundleID(nil)
        }
    }

    private func finishHide() {
        clearMenuTarget()
        rootView?.saveCurrentSession()
        rootView?.resetHomeExpansion()
        rootView?.stopPermissionPolling()
        rootView?.stopPerformanceSampling()
        rootView?.setHomeProvidersActive(false)
        let duration = MotionTokens.panelHideDuration
        NSAnimationContext.runAnimationGroup { context in
            context.duration = duration
            context.timingFunction = CAMediaTimingFunction(name: .easeIn)
            panel.animator().alphaValue = 0
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                self.panel.orderOut(nil)
                self.panel.alphaValue = 1
                self.onDidHide?()
            }
        }
    }

    func hideImmediatelyForAction() {
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            self.clearMenuTarget()
            self.rootView?.resetForActionDismiss()
            self.rootView?.resetHomeExpansion()
            self.rootView?.stopPermissionPolling()
            self.rootView?.stopPerformanceSampling()
            self.rootView?.setHomeProvidersActive(false)
            self.panel.orderOut(nil)
            self.panel.alphaValue = 1
            self.onDidHide?()
        }
    }

    func hideIfShowingForExternalActivation(bundleID: String?) {
        guard panel.isVisible else { return }
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
        panel.position(on: screen.visibleFrame)
    }
}
