import Foundation
import OSLog

enum LatencyTelemetry {
    private static let logger = Logger(subsystem: "app.luma", category: "latency")

    static func report(p95Milliseconds: Double) {
        #if DEBUG
        logger.debug("p95: \(Int(p95Milliseconds))ms")
        #endif
    }
}
