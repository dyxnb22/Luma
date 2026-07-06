@preconcurrency import AppKit

/// Host for cross-fading launcher regions — suppresses hit tests while transparent or disabled.
final class LauncherOverlayHostView: NSView {
    nonisolated(unsafe) var passesHitTests = true

    nonisolated override func hitTest(_ point: NSPoint) -> NSView? {
        guard passesHitTests, alphaValue > 0.01, !isHidden else { return nil }
        return super.hitTest(point)
    }
}
