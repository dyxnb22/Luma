import Foundation

/// Kind of object surfaced in the personal workbench.
public enum WorkbenchEntityKind: String, Sendable, Codable, CaseIterable {
    case project
    case note
    case todo
    case snippet
    case quicklink
    case clipboardItem
    case urlReference
    case fileReference
    case secretReference
}

/// Lightweight reference to a workbench object without migrating module-internal models.
public struct WorkbenchEntityRef: Sendable, Hashable, Codable {
    public let kind: WorkbenchEntityKind
    public let entityID: String
    public let moduleID: ModuleIdentifier
    public let title: String
    public let subtitle: String?

    public init(
        kind: WorkbenchEntityKind,
        entityID: String,
        moduleID: ModuleIdentifier,
        title: String,
        subtitle: String? = nil
    ) {
        self.kind = kind
        self.entityID = entityID
        self.moduleID = moduleID
        self.title = title
        self.subtitle = subtitle
    }
}
