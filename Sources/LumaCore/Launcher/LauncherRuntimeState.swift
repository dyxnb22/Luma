import Foundation

/// Process-wide launcher health flags for doctor and diagnostics export.
public enum LauncherRuntimeState {
    private static let lock = NSLock()
    nonisolated(unsafe) private static var _hotkeyRegistered = false
    nonisolated(unsafe) private static var _warmupTimeoutCount = 0

    public static var hotkeyRegistered: Bool {
        get {
            lock.lock()
            defer { lock.unlock() }
            return _hotkeyRegistered
        }
        set {
            lock.lock()
            defer { lock.unlock() }
            _hotkeyRegistered = newValue
        }
    }

    public static var warmupTimeoutCount: Int {
        get {
            lock.lock()
            defer { lock.unlock() }
            return _warmupTimeoutCount
        }
        set {
            lock.lock()
            defer { lock.unlock() }
            _warmupTimeoutCount = newValue
        }
    }

    public static func incrementWarmupTimeouts() {
        lock.lock()
        defer { lock.unlock() }
        _warmupTimeoutCount += 1
    }
}
