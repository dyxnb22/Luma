import Foundation

public struct ModuleContext: Sendable {
    public let runtime: ModuleRuntimeClients
    public let platform: PlatformClients
    public let launcherUI: any LauncherUIClient

    public init(
        runtime: ModuleRuntimeClients,
        platform: PlatformClients,
        launcherUI: any LauncherUIClient = NoopLauncherUIClient()
    ) {
        self.runtime = runtime
        self.platform = platform
        self.launcherUI = launcherUI
    }

    public init(
        logger: any LoggingClient,
        metrics: any MetricsClient,
        database: any DatabaseClient,
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        fileSystem: any FileSystemClient,
        translation: any TranslationClient,
        config: any ConfigurationClient,
        workspace: any WorkspaceClient = NoopWorkspaceClient(),
        clipboardSnapshot: any ClipboardSnapshotClient = NoopClipboardSnapshotClient(),
        launcherUI: any LauncherUIClient = NoopLauncherUIClient(),
        processMemory: any ProcessMemoryClient = NoopProcessMemoryClient(),
        reminders: any RemindersClient = NoopRemindersClient(),
        scriptRunner: any ScriptRunnerClient = NoopScriptRunnerClient(),
        notifications: any NotificationClient = NoopNotificationClient(),
        currentProject: any CurrentProjectClient = NoopCurrentProjectClient(),
        selectionSnapshot: any SelectionSnapshotClient = NoopSelectionSnapshotClient(),
        menuBarTree: any MenuBarTreeClient = NoopMenuBarTreeClient()
    ) {
        self.runtime = ModuleRuntimeClients(
            logger: logger,
            metrics: metrics,
            database: database,
            config: config
        )
        self.platform = PlatformClients(
            pasteboard: pasteboard,
            accessibility: accessibility,
            fileSystem: fileSystem,
            translation: translation,
            workspace: workspace,
            clipboardSnapshot: clipboardSnapshot,
            processMemory: processMemory,
            reminders: reminders,
            scriptRunner: scriptRunner,
            notifications: notifications,
            currentProject: currentProject,
            selectionSnapshot: selectionSnapshot,
            menuBarTree: menuBarTree
        )
        self.launcherUI = launcherUI
    }
}

public struct QueryContext: Sendable {
    public let deadline: ContinuousClock.Instant
    public let platform: QueryPlatformClients

    public init(
        deadline: ContinuousClock.Instant,
        platform: QueryPlatformClients = QueryPlatformClients()
    ) {
        self.deadline = deadline
        self.platform = platform
    }
}

public struct ActionContext: Sendable {
    public let runtime: ActionRuntimeClients
    public let platform: ActionPlatformClients
    public let host: any HostClient
    public let launcherUI: any LauncherUIClient

    public init(
        runtime: ActionRuntimeClients,
        platform: ActionPlatformClients,
        host: any HostClient = NoopHostClient(),
        launcherUI: any LauncherUIClient = NoopLauncherUIClient()
    ) {
        self.runtime = runtime
        self.platform = platform
        self.host = host
        self.launcherUI = launcherUI
    }

    public init(
        logger: any LoggingClient,
        metrics: any MetricsClient,
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        translation: any TranslationClient = NoopTranslationClient(),
        workspace: any WorkspaceClient = NoopWorkspaceClient(),
        host: any HostClient = NoopHostClient(),
        launcherUI: any LauncherUIClient = NoopLauncherUIClient(),
        scriptRunner: any ScriptRunnerClient = NoopScriptRunnerClient(),
        currentProject: any CurrentProjectClient = NoopCurrentProjectClient(),
        selectionSnapshot: any SelectionSnapshotClient = NoopSelectionSnapshotClient()
    ) {
        self.runtime = ActionRuntimeClients(logger: logger, metrics: metrics)
        self.platform = ActionPlatformClients(
            pasteboard: pasteboard,
            accessibility: accessibility,
            translation: translation,
            workspace: workspace,
            scriptRunner: scriptRunner,
            currentProject: currentProject,
            selectionSnapshot: selectionSnapshot
        )
        self.host = host
        self.launcherUI = launcherUI
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
    func writeImage(data: Data, pasteboardType: String) async
    func writeFileURLs(_ urls: [URL]) async
    func readString() async -> String?
}

public protocol AccessibilityClient: Sendable {
    func isTrusted() async -> Bool
    func requestPermission() async
    func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async
    func insert(text: String) async
    func replaceSelectedText(with text: String) async -> Bool
    func applyWindowLayout(_ preset: String) async
}

public protocol FileSystemClient: Sendable {
    func watch(root: URL, debounceMillis: Int) async -> AsyncStream<[FSChangeEvent]>
    func stopWatching(root: URL) async
}

public protocol TranslationClient: Sendable {
    func translate(_ text: String) async throws -> TranslationOutcome
}

public protocol ConfigurationClient: Sendable {
    func enabledModules() async -> Set<ModuleIdentifier>?
    func clipboardMaxEntries() async -> Int
    func clipboardMaxAgeDays() async -> Int
    func clipboardMaxEntrySizeKB() async -> Int
    func clipboardHistoryEnabled() async -> Bool
    func clipboardIgnoredBundleIDs() async -> [String]
    func clipboardPasteBehavior() async -> String
    func translationTargetLanguage() async -> String
    func secretsAutoClearSeconds() async -> Int
    func secretsRelockTimeoutSeconds() async -> Int
    func secretsRequireUnlockOnLaunch() async -> Bool
}
