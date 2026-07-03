import AppKit
import LumaCore

@MainActor
final class LauncherPanel: NSPanel {
    var onEscape: (() -> Void)?
    /// Forwards module detail shortcuts when detail subviews hold focus.
    var onDetailKeyDown: ((NSEvent) -> Bool)?
    /// ⌘W close detail — return true when handled.
    var onCloseDetail: (() -> Bool)?

    /// Frame size locked after `position(on:)` — in-panel relayout must not widen the panel.
    private(set) var lockedFrameSize: NSSize?

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
        position(on: visibleFrame)
    }

    /// Sizes and centers the panel within a screen's visible frame (atomic `setFrame`).
    func position(on visibleFrame: NSRect) {
        let contentSize = LauncherPanelGeometry.contentSize(for: visibleFrame)
        let contentRect = NSRect(origin: .zero, size: contentSize)
        let frameSize = frameRect(forContentRect: contentRect).size
        let origin = NSPoint(
            x: round(visibleFrame.midX - frameSize.width / 2),
            y: round(visibleFrame.minY + (visibleFrame.height - frameSize.height) * LauncherChromeTokens.panelVerticalBias)
        )
        let frame = NSRect(origin: origin, size: frameSize)
        lockedFrameSize = frameSize
        minSize = frameSize
        maxSize = frameSize
        setFrame(frame, display: true)
    }

    /// Re-locks to the current presentation screen when home split needs the full default width (ADR-032).
    func ensureFitsHomeSplitLayout(on visibleFrame: NSRect) {
        let desired = LauncherPanelGeometry.contentSize(for: visibleFrame)
        let locked = lockedFrameSize ?? .zero
        guard locked.width < desired.width - 0.5 || locked.height < desired.height - 0.5 else { return }
        position(on: visibleFrame)
    }

    /// Keeps in-panel Auto Layout in sync after frame or content changes.
    func stabilizeContentLayout() {
        guard let contentView else { return }
        let layoutRect = contentLayoutRect
        if contentView.frame != layoutRect {
            contentView.frame = layoutRect
        }
        contentView.layoutSubtreeIfNeeded()
    }

    /// Re-centers and re-locks size when AppKit drifts the panel after nested relayout (help list, module prefix).
    func enforceLockedGeometry(using visibleFrame: NSRect? = nil) {
        guard lockedFrameSize != nil else { return }
        guard let screen = self.screen ?? LumaPresentationScreen.current() else { return }
        let visible = visibleFrame ?? screen.visibleFrame
        let expectedX = round(visible.midX - frame.width / 2)
        let sizeDrifted = lockedFrameSize.map { locked in
            abs(frame.width - locked.width) > 0.5 || abs(frame.height - locked.height) > 0.5
        } ?? false
        let xDrifted = abs(frame.origin.x - expectedX) > 1
        guard sizeDrifted || xDrifted else {
            stabilizeContentLayout()
            return
        }
        var corrected = frame
        if let locked = lockedFrameSize {
            corrected.size = locked
        }
        corrected.origin.x = expectedX
        setFrame(corrected, display: true)
    }

    /// Backward-compatible entry point for horizontal-only drift checks.
    func recenterIfHorizontallyDrifted(using visibleFrame: NSRect? = nil) {
        enforceLockedGeometry(using: visibleFrame)
    }

    override func setFrame(_ frameRect: NSRect, display displayFlag: Bool) {
        super.setFrame(clampedFrame(frameRect), display: displayFlag)
        stabilizeContentLayout()
    }

    override func setContentSize(_ size: NSSize) {
        if let locked = lockedFrameSize {
            super.setContentSize(locked)
        } else {
            super.setContentSize(size)
        }
        stabilizeContentLayout()
    }

    private func clampedFrame(_ proposed: NSRect) -> NSRect {
        guard let locked = lockedFrameSize else { return proposed }
        var rect = proposed
        if abs(rect.width - locked.width) > 0.5 || abs(rect.height - locked.height) > 0.5 {
            rect.size = locked
        }
        return rect
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if LumaStandardEditShortcuts.performKeyEquivalent(event, in: self) {
            return true
        }
        if onDetailKeyDown?(event) == true { return true }
        if closeDetailIfCommandW(event) { return true }
        return super.performKeyEquivalent(with: event)
    }

    override func keyDown(with event: NSEvent) {
        if onDetailKeyDown?(event) == true { return }
        if closeDetailIfCommandW(event) { return }
        if event.keyCode == 53 {
            onEscape?()
            return
        }
        super.keyDown(with: event)
    }

    private func closeDetailIfCommandW(_ event: NSEvent) -> Bool {
        guard event.modifierFlags.contains(.command),
              event.charactersIgnoringModifiers?.lowercased() == "w" else { return false }
        return onCloseDetail?() ?? false
    }
}
