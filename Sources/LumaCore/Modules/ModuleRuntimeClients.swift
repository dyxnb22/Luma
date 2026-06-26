import Foundation

public struct ModuleRuntimeClients: Sendable {
    public let logger: any LoggingClient
    public let metrics: any MetricsClient
    public let database: any DatabaseClient
    public let config: any ConfigurationClient

    public init(
        logger: any LoggingClient,
        metrics: any MetricsClient,
        database: any DatabaseClient,
        config: any ConfigurationClient
    ) {
        self.logger = logger
        self.metrics = metrics
        self.database = database
        self.config = config
    }
}

public struct ActionRuntimeClients: Sendable {
    public let logger: any LoggingClient
    public let metrics: any MetricsClient

    public init(logger: any LoggingClient, metrics: any MetricsClient) {
        self.logger = logger
        self.metrics = metrics
    }
}
