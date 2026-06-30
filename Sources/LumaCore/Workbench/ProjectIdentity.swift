import CryptoKit
import Foundation

/// Stable project identity for activity queries and entity links (v2).
public struct ProjectIdentity: Sendable, Hashable, Codable {
    public let stableProjectID: String
    public let matchedPath: String?
    public let labelFallback: String
    public let displayName: String?
    public let sourceBundleID: String?

    public init(
        stableProjectID: String,
        matchedPath: String?,
        labelFallback: String,
        displayName: String? = nil,
        sourceBundleID: String? = nil
    ) {
        self.stableProjectID = stableProjectID
        self.matchedPath = matchedPath
        self.labelFallback = labelFallback
        self.displayName = displayName
        self.sourceBundleID = sourceBundleID
    }

    public init(context: CurrentProjectContext) {
        let path = context.matchedProjectPath
        let label = context.projectLabel
        let bundle = context.bundleID
        stableProjectID = Self.makeStableID(matchedPath: path, labelFallback: label, sourceBundleID: bundle)
        matchedPath = path
        labelFallback = label
        displayName = context.projectName
        sourceBundleID = bundle
    }

    public init(legacy: WorkbenchProjectAssociation, sourceBundleID: String? = nil) {
        let matchedPath: String?
        let labelFallback: String
        if Self.looksLikePath(legacy.projectPath) {
            matchedPath = legacy.projectPath
            labelFallback = legacy.projectLabel
        } else {
            matchedPath = nil
            labelFallback = legacy.projectPath
        }
        self.matchedPath = matchedPath
        self.labelFallback = labelFallback
        displayName = legacy.projectName ?? legacy.projectLabel
        self.sourceBundleID = sourceBundleID
        stableProjectID = Self.makeStableID(
            matchedPath: matchedPath,
            labelFallback: labelFallback,
            sourceBundleID: sourceBundleID
        )
    }

    /// Query key for legacy path/label string matching (fallback only).
    public var activityQueryKey: String? {
        if let path = matchedPath?.trimmingCharacters(in: .whitespacesAndNewlines), !path.isEmpty {
            return path
        }
        let label = labelFallback.trimmingCharacters(in: .whitespacesAndNewlines)
        return label.isEmpty ? nil : label
    }

    public static func makeStableID(
        matchedPath: String?,
        labelFallback: String,
        sourceBundleID: String?
    ) -> String {
        if let path = matchedPath?.trimmingCharacters(in: .whitespacesAndNewlines), !path.isEmpty {
            return "path:" + sha256(normalizePath(path))
        }
        let label = labelFallback.trimmingCharacters(in: .whitespacesAndNewlines)
        let bundle = sourceBundleID?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if bundle.isEmpty {
            return "label-legacy:" + sha256(label)
        }
        return "label:" + sha256("\(label)|\(bundle)")
    }

    public static func isLegacyLabelStableID(_ stableProjectID: String) -> Bool {
        stableProjectID.hasPrefix("label-legacy:")
    }

    public static func looksLikePath(_ value: String) -> Bool {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.hasPrefix("/") || trimmed.hasPrefix("~")
    }

    public static func normalizePath(_ path: String) -> String {
        var normalized = path.trimmingCharacters(in: .whitespacesAndNewlines)
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        if normalized.hasPrefix("~") {
            normalized = home + normalized.dropFirst()
        }
        while normalized.count > 1, normalized.hasSuffix("/") {
            normalized.removeLast()
        }
        return normalized
    }

    private static func sha256(_ text: String) -> String {
        let digest = SHA256.hash(data: Data(text.utf8))
        return digest.map { String(format: "%02x", $0) }.joined()
    }
}
