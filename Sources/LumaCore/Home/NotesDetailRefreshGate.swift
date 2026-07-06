import Foundation

/// Generation guard for Notes detail tree reloads — stale async work must not write UI.
public struct NotesDetailRefreshGate: Sendable {
    private var generation = CancellationGeneration()

    public init() {}

    /// Starts a new refresh generation; prior in-flight refreshes become stale.
    @discardableResult
    public mutating func beginRefresh() -> UInt {
        generation.bump()
    }

    /// Invalidates in-flight refresh work (deactivate, hide, close detail).
    public mutating func invalidate() {
        _ = generation.bump()
    }

    public func isCurrent(_ token: UInt) -> Bool {
        generation.isCurrent(token)
    }

    public var current: UInt { generation.current }
}
