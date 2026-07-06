import Foundation

public struct DiagnosticsPayload: Codable, Equatable, Sendable {
    public struct DurationSummary: Codable, Equatable, Sendable {
        public let p95Milliseconds: [String: Double]
    }

    public let generatedAt: String
    public let appVersion: String?
    public let buildNumber: String?
    public let latencyP95Milliseconds: Double?
    public let perfCounters: [String: Int]
    public let durationSummary: [String: Double]
    public let breadcrumbs: [String]

    public init(
        generatedAt: String,
        appVersion: String?,
        buildNumber: String?,
        latencyP95Milliseconds: Double?,
        perfCounters: [String: Int],
        durationSummary: [String: Double],
        breadcrumbs: [String]
    ) {
        self.generatedAt = generatedAt
        self.appVersion = appVersion
        self.buildNumber = buildNumber
        self.latencyP95Milliseconds = latencyP95Milliseconds
        self.perfCounters = perfCounters
        self.durationSummary = durationSummary
        self.breadcrumbs = breadcrumbs
    }
}

/// Local diagnostics export — no network, redacted payloads only.
public enum DiagnosticsExport {
    public static let defaultDirectoryName = "Luma"
    public static let defaultFileName = "diagnostics.json"

    public static func buildPayload(
        appVersion: String? = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String,
        buildNumber: String? = Bundle.main.infoDictionary?["CFBundleVersion"] as? String,
        latencyP95: Double? = nil,
        perfCounters: [String: Int] = LauncherPerfCounters.exportSnapshot(),
        durationSummary: [String: Double] = LauncherDurationRecorder.exportSummary(),
        breadcrumbs: [String] = []
    ) -> DiagnosticsPayload {
        DiagnosticsPayload(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            appVersion: appVersion,
            buildNumber: buildNumber,
            latencyP95Milliseconds: latencyP95,
            perfCounters: perfCounters,
            durationSummary: durationSummary,
            breadcrumbs: breadcrumbs.map(redactBreadcrumb)
        )
    }

  public static func exportToLogsDirectory(
        latencyP95: Double? = nil,
        breadcrumbs: [String] = []
    ) throws -> URL {
        let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/\(defaultDirectoryName)", isDirectory: true)
        guard let directory else {
            throw NSError(domain: "DiagnosticsExport", code: 1)
        }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent(defaultFileName)
        let payload = buildPayload(latencyP95: latencyP95, breadcrumbs: breadcrumbs)
        let data = try JSONEncoder().encode(payload)
        try data.write(to: url, options: .atomic)
        return url
    }

    /// Strips query-like, secret, and clipboard fragments from breadcrumb lines.
    public static func redactBreadcrumb(_ line: String) -> String {
        var redacted = line
        let patterns = [
            #"query=.*"#,
            #"clipboard=.*"#,
            #"secret=.*"#,
            #"noteBody=.*"#,
            #"payload=.*"#
        ]
        for pattern in patterns {
            if let regex = try? NSRegularExpression(pattern: pattern, options: .caseInsensitive) {
                redacted = regex.stringByReplacingMatches(
                    in: redacted,
                    range: NSRange(redacted.startIndex..., in: redacted),
                    withTemplate: ""
                )
            }
        }
        return redacted.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
