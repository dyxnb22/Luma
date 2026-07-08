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
    private var panelHideTask: Task<Void, Never>?
    private var visibilitySession = LauncherPanelVisibilitySession()
    private var lastPositionedVisibleFrame: CGRect?
    private var lastToggleAt: ContinuousClock.Instant?
    private var lastCarbonShowAt: ContinuousClock.Instant?
    private var lastPanelHideAt: ContinuousClock.Instant?
    private var hideStart: ContinuousClock.Instant?
    private var notificationObservers: [NSObjectProtocol] = []
    private var qaCommandTimer: Timer?

    var isPanelVisible: Bool { visibilitySession.isVisible }

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
            self?.hideFromVisibleHotkey()
        }
        panel.orderOut(nil)
        notificationObservers.append(
            LumaNotificationCenter.observe(name: NSApplication.didChangeScreenParametersNotification) { [weak self] in
                self?.repositionPanelIfVisible()
            }
        )
        notificationObservers.append(
            NSWorkspace.shared.notificationCenter.addObserver(
                forName: NSWorkspace.activeSpaceDidChangeNotification,
                object: nil,
                queue: nil
            ) { [weak self] _ in
                Task { @MainActor in
                    self?.repositionPanelIfVisible()
                }
            }
        )
        installQACommandPollerIfNeeded()
    }

    private func installQACommandPollerIfNeeded() {
        guard ProcessInfo.processInfo.environment["LUMA_QA"] == "1" else { return }
        qaCommandTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.consumeQACommandIfNeeded()
            }
        }
    }

    private func consumeQACommandIfNeeded() {
        guard let url = Self.qaCommandURL(),
              FileManager.default.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url) else { return }
        guard let rootView else { return }
        try? FileManager.default.removeItem(at: url)
        guard let command = try? JSONDecoder().decode(QACommand.self, from: data) else { return }
        switch command.command {
        case "bareOpen":
            _ = rootView.performQABareCommandAction(raw: command.raw)
        default:
            break
        }
    }

    private static func qaCommandURL() -> URL? {
        FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Luma/qa-command.json")
    }

    private struct QACommand: Decodable {
        var command: String
        var raw: String
    }

    private func repositionPanelIfVisible() {
        guard let screen = LumaPresentationScreen.current() else { return }
        let visibleFrame = screen.visibleFrame
        guard LauncherPanelRepositionPolicy.shouldReposition(
            isPanelVisible: visibilitySession.isVisible,
            lastVisibleFrame: lastPositionedVisibleFrame,
            newVisibleFrame: visibleFrame
        ) else { return }
        positionPanel()
        panel.enforceLockedGeometry(using: visibleFrame)
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
        rootView.configurePanelSnapshotContext { [weak self] in
            guard let self else { return (false, 0, false) }
            return (
                self.visibilitySession.isVisible,
                self.visibilitySession.generation,
                self.panel.isVisible
            )
        }
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
        guard !visibilitySession.isVisible else { return }
        rootView?.refreshHome(intent: .backgroundCacheWarm)
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

    func handleModulesDisabled(removed: Set<ModuleIdentifier>) {
        rootView?.handleModulesDisabled(removed: removed)
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
        visibilitySession.isVisible ? hide() : show()
    }

    /// Carbon global hotkey — show only when panel is hidden.
    func showFromCarbonHotkey() {
        show(reason: .carbonHotkey)
    }

    /// Menu bar Show — may re-front/refocus when already visible (intentional; see `LAUNCHER_SHOW_ENTRY_CONTRACT.md`).
    func showFromMenuBar() {
        show(reason: .menuBar)
    }

    func show(reason: LauncherShowReason) {
        if visibilitySession.isVisible,
           !LauncherShowEntryPolicy.shouldBeginShowWhenAlreadyVisible(reason: reason) {
            return
        }
        if LauncherShowEntryPolicy.appliesCarbonShowDebounce(reason: reason) {
            let now = ContinuousClock.now
            if let lastCarbonShowAt, now - lastCarbonShowAt < .milliseconds(120) {
                return
            }
            lastCarbonShowAt = now
        }
        show()
    }

    /// Visible panel ⌘Space via `LauncherPanel.performKeyEquivalent` — hide only.
    func hideFromVisibleHotkey() {
        guard visibilitySession.isVisible else { return }
        let now = ContinuousClock.now
        if let lastPanelHideAt, now - lastPanelHideAt < .milliseconds(120) {
            return
        }
        lastPanelHideAt = now
        LauncherStateKeyboardRecorder.record("cmdSpaceHide")
        hide()
    }

    func show() {
        cancelDeferredShowWork()
        cancelPanelHideAnimation()
        let generation = visibilitySession.beginShow()

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

        DispatchQueue.main.async { [weak self] in
            guard let self, self.visibilitySession.shouldCompleteDeferredShow(generation: generation) else { return }
            self.rootView?.focusSearchFieldAfterShow()
            if ProcessInfo.processInfo.environment["LUMA_QA"] == "1" {
                self.ensureSearchFieldFocused()
            }
            self.rootView?.reconcileLauncherStateAfterShow()
            // Cache-warm shows do not call refreshHome(); close the hotkey sample at first interactive frame.
            _ = HomeLatencyTracker.markHomeRendered()
            self.rootView?.exportStateSnapshot(reason: "showCompleted")
        }

        // Heavier panel services stay off the hotkey→visible path.
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.visibilitySession.shouldCompleteDeferredShow(generation: generation) else { return }
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

    /// Stops an in-flight hide fade so a rapid show does not leave the panel transparent.
    private func cancelPanelHideAnimation() {
        panelHideTask?.cancel()
        panelHideTask = nil
        panel.animations.removeAll()
        NSAnimationContext.beginGrouping()
        NSAnimationContext.current.duration = 0
        NSAnimationContext.current.allowsImplicitAnimation = false
        panel.alphaValue = 1
        NSAnimationContext.endGrouping()
    }

    func hide() {
        guard let generationAtHide = visibilitySession.beginHide() else { return }
        HomeLatencyTracker.abandonPendingHotkeyMark()
        LauncherPerfCounters.increment(.panelHide)
        hideStart = ContinuousClock.now
        cancelDeferredShowWork()
        rootView?.cancelActiveQueryAndSnapshotApply()
        rootView?.cancelPendingRestore()
        let duration = MotionTokens.panelHideDuration
        panelHideTask?.cancel()
        panelHideTask = Task { @MainActor in
            async let prepared = self.rootView?.prepareDetailForHide()
            await withCheckedContinuation { continuation in
                NSAnimationContext.runAnimationGroup({ context in
                    context.duration = duration
                    context.timingFunction = CAMediaTimingFunction(name: .easeIn)
                    self.panel.animator().alphaValue = 0
                }, completionHandler: {
                    continuation.resume()
                })
            }
            await prepared
            guard !Task.isCancelled else { return }
            self.finishHide(generationAtHide: generationAtHide)
        }
    }

    private func finalizePanelHidden() {
        clearMenuTarget()
        rootView?.invalidatePanelSignalsCache()
        rootView?.flushPendingSessionWrites()
        rootView?.resetHomeExpansion()
        rootView?.stopPermissionPolling()
        rootView?.stopPerformanceSampling()
        rootView?.setPanelSignalsActive(false)
        panel.orderOut(nil)
        panel.alphaValue = 1
        onDidHide?()
    }

    private func clearMenuTarget() {
        LauncherMenuTarget.clear()
        Task {
            await MenuBarTreeService.shared.setLauncherContextBundleID(nil)
        }
    }

    private func finishHide(generationAtHide: UInt) {
        guard visibilitySession.shouldCompleteHide(generationAtHide: generationAtHide) else { return }
        if let hideStart {
            let ms = LauncherDurationRecorder.durationMilliseconds(ContinuousClock.now - hideStart)
            LauncherDurationRecorder.record(category: .panelHide, key: "panel", milliseconds: ms)
            self.hideStart = nil
        }
        rootView?.saveCurrentSession()
        rootView?.exportStateSnapshot(reason: "hideCompleted")
        finalizePanelHidden()
    }

    func hideImmediatelyForAction() {
        guard let generationAtHide = visibilitySession.beginHide() else { return }
        HomeLatencyTracker.abandonPendingHotkeyMark()
        LauncherPerfCounters.increment(.panelHide)
        cancelDeferredShowWork()
        rootView?.cancelActiveQueryAndSnapshotApply()
        rootView?.cancelPendingRestore()
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            guard self.visibilitySession.shouldCompleteHide(generationAtHide: generationAtHide) else { return }
            self.rootView?.resetForActionDismiss()
            self.finalizePanelHidden()
        }
    }

    func hideIfShowingForExternalActivation(bundleID: String?) {
        guard visibilitySession.isVisible else { return }
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
