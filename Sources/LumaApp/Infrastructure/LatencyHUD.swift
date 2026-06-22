import Foundation
import OSLog

@MainActor
final class LatencyTelemetry {
    static let shared = LatencyTelemetry()
    private let logger = Logger(subsystem: "app.luma", category: "latency")
    private var samples: [Double] = []
    private let capacity = 200

    static func report(p95Milliseconds: Double) {
        shared.record(p95Milliseconds)
    }

    func record(_ ms: Double) {
        samples.append(ms)
        if samples.count > capacity { samples.removeFirst(samples.count - capacity) }
        #if DEBUG
        if samples.count % 20 == 0 {
            logger.debug("Rolling p95: \(Int(self.currentP95()))ms over \(self.samples.count) samples")
        }
        #endif
    }

    func currentP95() -> Double {
        guard !samples.isEmpty else { return 0 }
        let sorted = samples.sorted()
        let idx = Int(Double(sorted.count) * 0.95)
        return sorted[min(idx, sorted.count - 1)]
    }

    func recentSamples() -> [Double] {
        samples
    }
}
