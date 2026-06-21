import Foundation
import LumaCore
import OSLog

public struct LumaLogger: LoggingClient {
    private let logger: Logger

    public init(subsystem: String = "app.luma", category: String = "general") {
        self.logger = Logger(subsystem: subsystem, category: category)
    }

    public func debug(_ message: String) async {
        logger.debug("\(message, privacy: .public)")
    }

    public func error(_ message: String) async {
        logger.error("\(message, privacy: .public)")
    }
}
