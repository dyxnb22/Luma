import Foundation

/// Encodes opening a linked workbench entity (distinct from resuming a draft activity).
public enum WorkbenchEntityAction: Codable, Sendable, Equatable {
    case openLinked(linkID: UUID)
    case openActivityEntry(entryID: UUID)
    case showStatus(String)
}
