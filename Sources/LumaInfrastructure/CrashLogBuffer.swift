import Foundation
import LumaCore

/// Local ring buffer for recent crash/error messages (no network telemetry).
public actor CrashLogBuffer {
    public static let shared = CrashLogBuffer()

    /// On-disk breadcrumb file used by support (`~/Library/Application Support/Luma/crash-log.txt`).
    public static var standardFileURL: URL? {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Luma/crash-log.txt")
    }

    private var entries: [String] = []
    private let capacity = 50
    private var lastPersistFailed = false

    public func record(_ message: String) {
        let redacted = DiagnosticsExport.redactBreadcrumb(message)
        let stamp = ISO8601DateFormatter().string(from: Date())
        entries.append("[\(stamp)] \(redacted)")
        if entries.count > capacity {
            entries.removeFirst(entries.count - capacity)
        }
        persist()
    }

    public func all() -> [String] {
        entries
    }

    /// `available`, `missing`, `writeFailed`, or `unavailable`.
    public func fileWriteStatus() -> String {
        guard let url = Self.standardFileURL else { return "unavailable" }
        if lastPersistFailed { return "writeFailed" }
        if FileManager.default.fileExists(atPath: url.path) { return "available" }
        return entries.isEmpty ? "missing" : "writeFailed"
    }

    private func persist() {
        let url = Self.standardFileURL
        guard let url else {
            lastPersistFailed = true
            return
        }
        do {
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            try entries.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
            lastPersistFailed = false
        } catch {
            lastPersistFailed = true
        }
    }
}
