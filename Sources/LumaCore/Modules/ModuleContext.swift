import Foundation

public struct ModuleContext: Sendable {
    public let logger: any LoggingClient
    public let metrics: any MetricsClient
    public let database: any DatabaseClient
    public let pasteboard: any PasteboardClient
    public let accessibility: any AccessibilityClient
    public let fileSystem: any FileSystemClient
    public let translation: any TranslationClient
    public let config: any ConfigurationClient

    public init(
        logger: any LoggingClient,
        metrics: any MetricsClient,
        database: any DatabaseClient,
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        fileSystem: any FileSystemClient,
        translation: any TranslationClient,
        config: any ConfigurationClient
    ) {
        self.logger = logger
        self.metrics = metrics
        self.database = database
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.fileSystem = fileSystem
        self.translation = translation
        self.config = config
    }
}

public struct QueryContext: Sendable {
    public let deadline: ContinuousClock.Instant

    public init(deadline: ContinuousClock.Instant) {
        self.deadline = deadline
    }
}

public struct ActionContext: Sendable {
    public let logger: any LoggingClient
    public let metrics: any MetricsClient
    public let pasteboard: any PasteboardClient
    public let accessibility: any AccessibilityClient

    public init(
        logger: any LoggingClient,
        metrics: any MetricsClient,
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient
    ) {
        self.logger = logger
        self.metrics = metrics
        self.pasteboard = pasteboard
        self.accessibility = accessibility
    }
}

public protocol LoggingClient: Sendable {
    func debug(_ message: String) async
    func error(_ message: String) async
}

public protocol MetricsClient: Sendable {
    func mark(_ name: String) async
}

public struct NoopMetricsClient: MetricsClient {
    public init() {}
    public func mark(_ name: String) async {
        _ = name
    }
}

/// Currently a path provider for Application Support; may grow into a real SQLite client later.
public protocol DatabaseClient: Sendable {}

public protocol PasteboardClient: Sendable {
    func write(_ string: String) async
    func writeSecure(_ string: String, clearAfterSeconds: Int) async
}

public protocol AccessibilityClient: Sendable {
    func focus(windowID: UInt32, pid: Int32, title: String) async
    func insert(text: String) async
    func applyWindowLayout(_ preset: String) async
}

public protocol FileSystemClient: Sendable {}

public protocol TranslationClient: Sendable {
    func translate(_ text: String) async throws -> TranslationOutcome
}

public protocol ConfigurationClient: Sendable {
    func enabledModules() async -> Set<ModuleIdentifier>?
    func clipboardMaxEntries() async -> Int
    func clipboardMaxAgeDays() async -> Int
    func clipboardMaxEntrySizeKB() async -> Int
    func translationTargetLanguage() async -> String
    func secretsAutoClearSeconds() async -> Int
    func secretsRelockTimeoutSeconds() async -> Int
    func secretsRequireUnlockOnLaunch() async -> Bool
}
