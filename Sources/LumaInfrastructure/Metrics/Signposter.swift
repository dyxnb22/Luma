import Foundation
import LumaCore
import OSLog

public struct LumaMetrics: MetricsClient {
    private let logger: Logger
    private let signposter: OSSignposter

    public init(subsystem: String = "app.luma", category: String = "latency") {
        self.logger = Logger(subsystem: subsystem, category: category)
        self.signposter = OSSignposter(subsystem: subsystem, category: category)
    }

    public func mark(_ name: String) async {
        logger.debug("metric: \(name, privacy: .public)")
        signposter.emitEvent("metric")
    }
}
