import Foundation

/// Holds an `NSObjectProtocol` observer token for use inside actors without `nonisolated(unsafe)`.
public final class NotificationObserverBox: @unchecked Sendable {
    private let lock = NSLock()
    private var token: NSObjectProtocol?

    public init() {}

    public func set(_ observer: NSObjectProtocol?) {
        lock.lock()
        defer { lock.unlock() }
        if let token {
            NotificationCenter.default.removeObserver(token)
        }
        token = observer
    }

    public func clear() {
        set(nil)
    }

    deinit {
        clear()
    }
}
