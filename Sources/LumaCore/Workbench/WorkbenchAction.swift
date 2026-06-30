import Foundation

/// Semantic workbench action kinds, bridged to `Action` at execution time.
public enum WorkbenchActionKind: String, Sendable, Codable, CaseIterable {
    case open
    case continueWork
    case create
    case capture
    case pin
    case archive
    case convert
    case linkToProject
    case prepareDraft
}

public struct WorkbenchAction: Sendable, Hashable {
    public let kind: WorkbenchActionKind
    public let targetModule: ModuleIdentifier
    public let title: String
    public let payload: Data?

    public init(
        kind: WorkbenchActionKind,
        targetModule: ModuleIdentifier,
        title: String,
        payload: Data? = nil
    ) {
        self.kind = kind
        self.targetModule = targetModule
        self.title = title
        self.payload = payload
    }
}
