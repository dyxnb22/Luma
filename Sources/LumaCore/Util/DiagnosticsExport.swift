import Foundation

public struct DiagnosticsPayload: Codable, Equatable, Sendable {
    public struct PlatformInfo: Codable, Equatable, Sendable {
        public let osVersion: String
        public let screenCount: Int
        public let presentationScreenName: String?

        public init(osVersion: String, screenCount: Int, presentationScreenName: String?) {
            self.osVersion = osVersion
            self.screenCount = screenCount
            self.presentationScreenName = presentationScreenName
        }
    }

    public struct ModuleInfo: Codable, Equatable, Sendable {
        public let enabledCount: Int
        public let totalCount: Int
        public let defaultEnabledCount: Int
        /// Enabled module identifiers at export time (sorted).
        public let enabledModuleIDs: [String]
        /// MVP P0 core modules and whether each is enabled at export time.
        public let mvpCoreModuleStatus: [MVPCoreModuleStatus]

        public init(
            enabledCount: Int,
            totalCount: Int,
            defaultEnabledCount: Int,
            enabledModuleIDs: [String] = [],
            mvpCoreModuleStatus: [MVPCoreModuleStatus] = []
        ) {
            self.enabledCount = enabledCount
            self.totalCount = totalCount
            self.defaultEnabledCount = defaultEnabledCount
            self.enabledModuleIDs = enabledModuleIDs
            self.mvpCoreModuleStatus = mvpCoreModuleStatus
        }
    }

    public struct MVPCoreModuleStatus: Codable, Equatable, Sendable {
        public let moduleID: String
        public let enabled: Bool

        public init(moduleID: String, enabled: Bool) {
            self.moduleID = moduleID
            self.enabled = enabled
        }
    }

    public struct PermissionsInfo: Codable, Equatable, Sendable {
        public let accessibilityTrusted: Bool
        public let remindersAuthorization: String?
        public let hotkeyRegistered: Bool

        public init(
            accessibilityTrusted: Bool,
            remindersAuthorization: String?,
            hotkeyRegistered: Bool
        ) {
            self.accessibilityTrusted = accessibilityTrusted
            self.remindersAuthorization = remindersAuthorization
            self.hotkeyRegistered = hotkeyRegistered
        }
    }

    public let generatedAt: String
    public let appVersion: String?
    public let buildNumber: String?
    public let latencyP95Milliseconds: Double?
    public let perfCounters: [String: Int]
    public let durationSummary: [String: Double]
    public let breadcrumbs: [String]
    public let platform: PlatformInfo?
    public let modules: ModuleInfo?
    public let permissions: PermissionsInfo?
    public let recentErrors: [String]
    public let corruptConfigFiles: [String]
    /// `~/Library/Application Support/Luma/crash-log.txt` when known.
    public let crashLogPath: String?
    /// `available`, `missing`, `writeFailed`, or `unavailable` when path cannot be resolved.
    public let crashLogWriteStatus: String?

    public init(
        generatedAt: String,
        appVersion: String?,
        buildNumber: String?,
        latencyP95Milliseconds: Double?,
        perfCounters: [String: Int],
        durationSummary: [String: Double],
        breadcrumbs: [String],
        platform: PlatformInfo? = nil,
        modules: ModuleInfo? = nil,
        permissions: PermissionsInfo? = nil,
        recentErrors: [String] = [],
        corruptConfigFiles: [String] = [],
        crashLogPath: String? = nil,
        crashLogWriteStatus: String? = nil
    ) {
        self.generatedAt = generatedAt
        self.appVersion = appVersion
        self.buildNumber = buildNumber
        self.latencyP95Milliseconds = latencyP95Milliseconds
        self.perfCounters = perfCounters
        self.durationSummary = durationSummary
        self.breadcrumbs = breadcrumbs
        self.platform = platform
        self.modules = modules
        self.permissions = permissions
        self.recentErrors = recentErrors
        self.corruptConfigFiles = corruptConfigFiles
        self.crashLogPath = crashLogPath
        self.crashLogWriteStatus = crashLogWriteStatus
    }
}

/// Local diagnostics export — no network, redacted payloads only.
public enum DiagnosticsExport {
    public static let defaultDirectoryName = "Luma"
    public static let defaultFileName = "diagnostics.json"

    private static let sensitiveBreadcrumbKeys: Set<String> = [
        "query", "clipboard", "secret", "notebody", "payload",
        "token", "password", "apikey", "accesstoken", "refreshtoken",
        "file", "path", "url", "bundle", "title", "subtitle",
        "stderr", "email", "hostname"
    ]

    public static func buildPayload(
        appVersion: String? = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String,
        buildNumber: String? = Bundle.main.infoDictionary?["CFBundleVersion"] as? String,
        latencyP95: Double? = nil,
        perfCounters: [String: Int] = LauncherPerfCounters.exportSnapshot(),
        durationSummary: [String: Double] = LauncherDurationRecorder.exportSummary(),
        breadcrumbs: [String] = [],
        platform: DiagnosticsPayload.PlatformInfo? = nil,
        modules: DiagnosticsPayload.ModuleInfo? = nil,
        permissions: DiagnosticsPayload.PermissionsInfo? = nil,
        recentErrors: [String] = [],
        corruptConfigFiles: [String] = ConfigCorruptionRegistry.snapshot(),
        crashLogPath: String? = nil,
        crashLogWriteStatus: String? = nil
    ) -> DiagnosticsPayload {
        DiagnosticsPayload(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            appVersion: appVersion,
            buildNumber: buildNumber,
            latencyP95Milliseconds: latencyP95,
            perfCounters: perfCounters,
            durationSummary: durationSummary,
            breadcrumbs: breadcrumbs.map(redactBreadcrumb),
            platform: platform,
            modules: modules,
            permissions: permissions,
            recentErrors: recentErrors.map(redactBreadcrumb),
            corruptConfigFiles: corruptConfigFiles,
            crashLogPath: crashLogPath,
            crashLogWriteStatus: crashLogWriteStatus
        )
    }

    public static func writePayload(_ payload: DiagnosticsPayload) throws -> URL {
        let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/\(defaultDirectoryName)", isDirectory: true)
        guard let directory else {
            throw NSError(domain: "DiagnosticsExport", code: 1)
        }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent(defaultFileName)
        let data = try JSONEncoder().encode(payload)
        try data.write(to: url, options: .atomic)
        return url
    }

    public static func exportToLogsDirectory(
        latencyP95: Double? = nil,
        breadcrumbs: [String] = [],
        platform: DiagnosticsPayload.PlatformInfo? = nil,
        modules: DiagnosticsPayload.ModuleInfo? = nil,
        permissions: DiagnosticsPayload.PermissionsInfo? = nil,
        recentErrors: [String] = [],
        crashLogPath: String? = nil,
        crashLogWriteStatus: String? = nil
    ) throws -> URL {
        let payload = buildPayload(
            latencyP95: latencyP95,
            breadcrumbs: breadcrumbs,
            platform: platform,
            modules: modules,
            permissions: permissions,
            recentErrors: recentErrors,
            crashLogPath: crashLogPath,
            crashLogWriteStatus: crashLogWriteStatus
        )
        return try writePayload(payload)
    }

    /// Redacts known sensitive `key=value` fields and home-directory path fragments.
    public static func redactBreadcrumb(_ line: String) -> String {
        let pathRedacted = redactPathHeuristics(in: line)
        return pathRedacted.split(separator: " ", omittingEmptySubsequences: false).map { token -> String in
            redactSensitiveToken(String(token))
        }.joined(separator: " ")
    }

    private static func redactSensitiveToken(_ token: String) -> String {
        guard let separator = token.firstIndex(of: "=") else { return token }
        let key = String(token[token.startIndex..<separator]).lowercased()
        guard sensitiveBreadcrumbKeys.contains(key) else { return token }
        let keyPart = token[token.startIndex..<separator]
        return "\(keyPart)=<redacted>"
    }

    private static func redactPathHeuristics(in text: String) -> String {
        var result = text
        let pathContinuation = #"(?:\s+[^\s"]*\/[^\s"]*)*"#
        if result.contains("~/") {
            result = result.replacingOccurrences(
                of: #"~/[^\s"]+"# + pathContinuation,
                with: "~/<redacted>",
                options: .regularExpression
            )
        }
        if result.contains("/Users/") {
            result = result.replacingOccurrences(
                of: #"/Users/[^\s"]+"# + pathContinuation,
                with: "/Users/<redacted>",
                options: .regularExpression
            )
        }
        return result
    }
}
