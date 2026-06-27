import Foundation

/// Synchronous launcher menu target captured before Luma becomes key.
public enum LauncherMenuTarget {
    private static let lock = NSLock()
    nonisolated(unsafe) private static var bundleID: String?

    public static func set(bundleID: String?) {
        lock.lock()
        defer { lock.unlock() }
        self.bundleID = bundleID
    }

    public static func current() -> String? {
        lock.lock()
        defer { lock.unlock() }
        return bundleID
    }

    public static func clear() {
        set(bundleID: nil)
    }
}
