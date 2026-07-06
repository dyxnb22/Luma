import Foundation

@MainActor
enum HomeLatencyTracker {
    private static var hotkeyMark: CFAbsoluteTime?

    static func markHotkey() {
        hotkeyMark = CFAbsoluteTimeGetCurrent()
    }

    /// Drop a pending hotkey sample when hide aborts the show→home path (prevents stale marks from attributing later background refresh).
    static func abandonPendingHotkeyMark() {
        hotkeyMark = nil
    }

    @discardableResult
    static func markHomeRendered() -> Double? {
        guard let start = hotkeyMark else { return nil }
        hotkeyMark = nil
        let ms = (CFAbsoluteTimeGetCurrent() - start) * 1000
        LatencyTelemetry.reportHotkey(ms)
        if ProcessInfo.processInfo.environment["LUMA_QA"] == "1" {
            _ = try? LatencyTelemetry.shared.exportReport()
        }
        return ms
    }
}
