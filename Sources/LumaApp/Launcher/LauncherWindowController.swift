import AppKit
import LumaCore

@MainActor
final class LauncherWindowController {
    private let panel = LauncherPanel()
    private var rootView: LauncherRootView?
    private var navigationDepth = 0

    init() {
        panel.onEscape = { [weak self] in
            self?.escape()
        }
        panel.orderOut(nil)
    }

    func configure(cards: [FeatureCard], cardLayoutStore: CardLayoutStore, viewModel: LauncherViewModel, actionExecutor: ActionExecutor) {
        let rootView = LauncherRootView(
            cards: cards,
            cardLayoutStore: cardLayoutStore,
            viewModel: viewModel,
            actionExecutor: actionExecutor,
            onDismiss: { [weak self] in self?.hide() },
            onOpenFeature: { [weak self] in self?.navigationDepth = 1 },
            onBackOrDismiss: { [weak self] in self?.escape() }
        )
        panel.contentView = rootView
        self.rootView = rootView
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
        } completionHandler: {
            Task { @MainActor in
                self.panel.orderOut(nil)
                self.rootView?.showHome()
                self.navigationDepth = 0
            }
        }
    }

    private func escape() {
        if navigationDepth > 0 {
            navigationDepth -= 1
            rootView?.showHome()
        } else {
            hide()
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
