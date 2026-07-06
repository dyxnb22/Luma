@preconcurrency import AppKit

final class FlippedStackView: NSStackView {
    nonisolated override var isFlipped: Bool { true }
}
