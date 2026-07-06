import Foundation

/// Monotonic generation counter for invalidating in-flight async work after cancel/hide.
public struct CancellationGeneration: Sendable {
    private var value: UInt = 0

    public init() {}

    public var current: UInt { value }

    @discardableResult
    public mutating func bump() -> UInt {
        value &+= 1
        return value
    }

    public func isCurrent(_ generation: UInt) -> Bool {
        value == generation
    }
}
