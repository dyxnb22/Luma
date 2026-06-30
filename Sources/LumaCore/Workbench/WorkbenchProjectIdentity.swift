import Foundation

/// Runtime project identity wrapper for workbench context building.
public struct WorkbenchProjectIdentity: Sendable, Equatable {
    public let identity: ProjectIdentity

    public init(context: CurrentProjectContext) {
        identity = ProjectIdentity(context: context)
    }

    public init(identity: ProjectIdentity) {
        self.identity = identity
    }

    public init(matchedPath: String?, labelFallback: String, sourceBundleID: String? = nil) {
        identity = ProjectIdentity(
            stableProjectID: ProjectIdentity.makeStableID(
                matchedPath: matchedPath,
                labelFallback: labelFallback,
                sourceBundleID: sourceBundleID
            ),
            matchedPath: matchedPath,
            labelFallback: labelFallback,
            displayName: labelFallback,
            sourceBundleID: sourceBundleID
        )
    }

    public var matchedPath: String? { identity.matchedPath }
    public var labelFallback: String { identity.labelFallback }
    public var stableProjectID: String { identity.stableProjectID }
    public var activityQueryKey: String? { identity.activityQueryKey }
}
