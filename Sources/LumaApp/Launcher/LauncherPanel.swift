import AppKit

@MainActor
final class LauncherPanel: NSPanel {
    var onEscape: (() -> Void)?

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 820, height: 560),
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
        let width = max(780, min(860, visibleFrame.width * 0.55))
        let height = max(520, min(620, visibleFrame.height * 0.62))
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
