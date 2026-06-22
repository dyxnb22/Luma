import AppKit
import LumaCore

@MainActor
final class LauncherWindowController {
    private let panel = LauncherPanel()
    private var rootView: LauncherRootView?

    init() {
        panel.onEscape = { [weak self] in
            // Route through rootView so detail→grid→close state machine is respected.
            // If rootView is not set yet, fall back to hide.
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
        cards: [FeatureCard],
        viewModel: LauncherViewModel,
        actionExecutor: ActionExecutor,
        appActivationTracker: AppActivationTracker
    ) {
        let rootView = LauncherRootView(
            cards: cards,
            viewModel: viewModel,
            actionExecutor: actionExecutor,
            appActivationTracker: appActivationTracker,
            onDismiss: { [weak self] in self?.hide() }
        )
        panel.contentView = rootView
        self.rootView = rootView
    }

    func refreshOpenApps() {
        rootView?.refreshOpenApps()
    }

    func setModulesReady(_ ready: Bool) {
        rootView?.setModulesReady(ready)
    }

    func toggle() {
        panel.isVisible ? hide() : show()
    }

    func show() {
        positionPanel()
        panel.alphaValue = 0
        panel.orderFrontRegardless()
        panel.makeKey()
        rootView?.focusSearchField()
        rootView?.refreshOpenApps()
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.16
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            panel.animator().alphaValue = 1
        }
    }

    func hide() {
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.12
            context.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
            panel.animator().alphaValue = 0
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                self.panel.orderOut(nil)
                self.panel.alphaValue = 1
                self.rootView?.showHome()
            }
        }
    }

    private func positionPanel() {
        let screen = NSScreen.main ?? NSScreen.screens.first
        guard let visible = screen?.visibleFrame else { return }
        let frame = panel.frame
        let x = visible.midX - frame.width / 2
        let y = visible.minY + visible.height * 0.62
        panel.setFrameOrigin(NSPoint(x: x, y: y))
    }
}
