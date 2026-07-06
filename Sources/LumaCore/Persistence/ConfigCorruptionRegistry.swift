import Foundation

/// Tracks config files quarantined during the current process for doctor / diagnostics.
public enum ConfigCorruptionRegistry {
    private static let lock = NSLock()
    nonisolated(unsafe) private static var files: [String] = []

    public static func record(fileName: String) {
        lock.lock()
        defer { lock.unlock() }
        guard !files.contains(fileName) else { return }
        files.append(fileName)
    }

    public static func snapshot() -> [String] {
        lock.lock()
        defer { lock.unlock() }
        return files
    }

    public static func resetForTesting() {
        lock.lock()
        defer { lock.unlock() }
        files = []
    }
}
