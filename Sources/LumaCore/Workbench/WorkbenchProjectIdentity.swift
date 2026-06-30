import Foundation

/// Separates matched filesystem path from window-label fallback for activity queries.
public struct WorkbenchProjectIdentity: Sendable, Equatable {
    public let matchedPath: String?
    public let labelFallback: String

    public init(context: CurrentProjectContext) {
        matchedPath = context.matchedProjectPath
        labelFallback = context.projectLabel
    }

    public init(matchedPath: String?, labelFallback: String) {
        self.matchedPath = matchedPath
        self.labelFallback = labelFallback
    }

    /// Query key for activity snapshot: matched path when known; otherwise label for unmatched/legacy rows.
    public var activityQueryKey: String? {
        if let path = matchedPath?.trimmingCharacters(in: .whitespacesAndNewlines), !path.isEmpty {
            return path
        }
        let label = labelFallback.trimmingCharacters(in: .whitespacesAndNewlines)
        return label.isEmpty ? nil : label
    }
}
