import Foundation

/// Show/hide generation state for the launcher panel window controller.
///
/// Every show or hide bumps generation so stale deferred show work and in-flight hide
/// completions are ignored after a rapid toggle.
public struct LauncherPanelVisibilitySession: Sendable {
    public private(set) var isVisible = false
    public private(set) var generation: UInt = 0

    public init() {}

    /// Marks the panel visible and returns a token for deferred show-side work.
    @discardableResult
    public mutating func beginShow() -> UInt {
        generation &+= 1
        isVisible = true
        return generation
    }

    /// Marks the panel hidden. Returns a hide-completion token, or `nil` when already hidden.
    @discardableResult
    public mutating func beginHide() -> UInt? {
        guard isVisible else { return nil }
        generation &+= 1
        isVisible = false
        return generation
    }

    public func shouldCompleteDeferredShow(generation: UInt) -> Bool {
        isVisible && self.generation == generation
    }

    public func shouldCompleteHide(generationAtHide: UInt) -> Bool {
        self.generation == generationAtHide
    }
}
