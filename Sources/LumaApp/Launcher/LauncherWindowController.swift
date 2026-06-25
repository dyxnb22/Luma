import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class LauncherWindowController {
    private let panel = LauncherPanel()
    private var rootView: LauncherRootView?

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
        onOpenSettings: @escaping () -> Void
    ) {
        let rootView = LauncherRootView(
            viewModel: viewModel,
            homeCoordinator: homeCoordinator,
            actionExecutor: actionExecutor,
            config: config,
            onDismiss: { [weak self] in self?.hide() },
            onActionDismiss: { [weak self] in self?.hideImmediatelyForAction() },
            onOpenSettings: onOpenSettings
        )
        panel.contentView = rootView
        rootView.layer?.anchorPoint = CGPoint(x: 0.5, y: 0.5)
        self.rootView = rootView
        AppleTranslationHost.shared.attach(to: rootView)
    }

    func refreshOpenApps() {
        rootView?.refreshOpenApps()
    }

    func setLatencyHUDEnabled(_ enabled: Bool) {
        rootView?.setLatencyHUDEnabled(enabled)
    }

    func setModulesReady(_ ready: Bool) {
        rootView?.setModulesReady(ready)
    }

    func closeDetailIfShowing() {
        rootView?.closeDetail()
    }

    func openModuleDetail(for moduleID: ModuleIdentifier) {
        rootView?.openModuleDetail(for: moduleID)
    }

    func toggle() {
        panel.isVisible ? hide() : show()
    }

    func show() {
        HomeLatencyTracker.markHotkey()
        positionPanel()
        panel.alphaValue = 0
        panel.contentView?.layer?.transform = CATransform3DMakeScale(0.96, 0.96, 1)
        panel.orderFrontRegardless()
        panel.makeKey()
        rootView?.refreshPermissionStatus()
        rootView?.startPermissionPollingIfNeeded()
        rootView?.restoreLastSessionIfNeeded()
        rootView?.focusSearchField()
        rootView?.refreshOpenApps()
        let duration = MotionTokens.panelShowDuration
        NSAnimationContext.runAnimationGroup { context in
            context.duration = duration
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            panel.animator().alphaValue = 1
            panel.contentView?.layer?.transform = CATransform3DIdentity
        }
    }

    func hide() {
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            self.finishHide()
        }
    }

    private func finishHide() {
        rootView?.saveCurrentSession()
        rootView?.resetOpenAppsExpansion()
        rootView?.stopPermissionPolling()
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
            }
        }
    }

    func hideImmediatelyForAction() {
        Task { @MainActor in
            await self.rootView?.prepareDetailForHide()
            self.rootView?.resetForActionDismiss()
            self.rootView?.resetOpenAppsExpansion()
            self.rootView?.stopPermissionPolling()
            self.panel.orderOut(nil)
            self.panel.alphaValue = 1
        }
    }

    func hideIfShowingForExternalActivation(bundleID: String?) {
        guard panel.isVisible else { return }
        guard bundleID != Bundle.main.bundleIdentifier else { return }
        hideImmediatelyForAction()
    }

    private func positionPanel() {
        let screen = NSScreen.main ?? NSScreen.screens.first
        guard let visible = screen?.visibleFrame else { return }
        panel.resizeForScreen(visible)
        let frame = panel.frame
        let x = visible.midX - frame.width / 2
        let y = visible.minY + (visible.height - frame.height) * LauncherChromeTokens.panelVerticalBias
        panel.setFrameOrigin(NSPoint(x: x, y: y))
    }
}
