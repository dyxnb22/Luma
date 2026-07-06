import Foundation

/// Optional sink for sanitized crash/error breadcrumbs. Wired from LumaApp at launch.
public enum CrashLogRecording {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var handler: @Sendable (String) -> Void = { _ in }

    public static func setHandler(_ handler: @escaping @Sendable (String) -> Void) {
        lock.lock()
        self.handler = handler
        lock.unlock()
    }

    public static func record(_ message: String) {
        lock.lock()
        let handler = handler
        lock.unlock()
        handler(message)
    }
}
