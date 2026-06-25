import Foundation

@MainActor
enum HomeLatencyTracker {
    private static var hotkeyMark: CFAbsoluteTime?

    static func markHotkey() {
        hotkeyMark = CFAbsoluteTimeGetCurrent()
    }

    @discardableResult
    static func markHomeRendered() -> Double? {
        guard let start = hotkeyMark else { return nil }
        hotkeyMark = nil
        let ms = (CFAbsoluteTimeGetCurrent() - start) * 1000
        LatencyTelemetry.report(p95Milliseconds: ms)
        return ms
    }
}
