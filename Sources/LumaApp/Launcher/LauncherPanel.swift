import AppKit
import LumaCore

@MainActor
final class LauncherPanel: NSPanel {
    var onEscape: (() -> Void)?

    init() {
        super.init(
            contentRect: NSRect(
                x: 0, y: 0,
                width: LauncherChromeTokens.defaultPanelWidth,
                height: LauncherChromeTokens.defaultPanelHeight
            ),
            styleMask: [.borderless, .nonactivatingPanel, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        isFloatingPanel = true
        level = .modalPanel
        collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary, .ignoresCycle]
        hidesOnDeactivate = false
        isMovableByWindowBackground = false
        hasShadow = true
        backgroundColor = .clear
        animationBehavior = .none
        isReleasedWhenClosed = false
        isOpaque = false
    }

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
    override var acceptsFirstResponder: Bool { true }

    override func cancelOperation(_ sender: Any?) {
        onEscape?()
    }

    func resizeForScreen(_ visibleFrame: NSRect) {
        let preferredWidth = max(
            LauncherChromeTokens.minPanelWidth,
            min(LauncherChromeTokens.maxPanelWidth, visibleFrame.width * LauncherChromeTokens.panelWidthScreenRatio)
        )
        let preferredHeight = max(
            LauncherChromeTokens.minPanelHeight,
            min(LauncherChromeTokens.maxPanelHeight, visibleFrame.height * LauncherChromeTokens.panelHeightScreenRatio)
        )
        let width = min(preferredWidth, visibleFrame.width)
        let height = min(preferredHeight, visibleFrame.height)
        setContentSize(NSSize(width: width, height: height))
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if LumaStandardEditShortcuts.performKeyEquivalent(event, in: self) {
            return true
        }
        return super.performKeyEquivalent(with: event)
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 {
            onEscape?()
            return
        }
        super.keyDown(with: event)
    }
}
