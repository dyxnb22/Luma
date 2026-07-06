@preconcurrency import AppKit

@MainActor
final class FlippedStackView: NSStackView {
    override var isFlipped: Bool { true }
}
