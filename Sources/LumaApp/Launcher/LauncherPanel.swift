import AppKit

@MainActor
final class LauncherPanel: NSPanel {
    var onEscape: (() -> Void)?

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 860, height: 540),
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
        let width = max(720, min(860, visibleFrame.width * 0.45))
        let height = max(480, min(540, visibleFrame.height * 0.55))
        setContentSize(NSSize(width: width, height: height))
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 {
            onEscape?()
            return
        }
        super.keyDown(with: event)
    }
}
