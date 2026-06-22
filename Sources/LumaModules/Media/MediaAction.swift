import Foundation

public enum MediaAction: Codable, Sendable {
    case openDetail
    case edit(id: UUID)
    case editDraft(MediaEditorDraft)
    case capture(MediaEditorDraft)
    case copy(id: UUID)
}
