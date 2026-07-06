import Foundation
import LumaCore

/// Local ring buffer for recent crash/error messages (no network telemetry).
public actor CrashLogBuffer {
    public static let shared = CrashLogBuffer()

    private var entries: [String] = []
    private let capacity = 50

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

    private func persist() {
        let url = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Luma/crash-log.txt")
        guard let url else { return }
        try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? entries.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
    }
}
