import Foundation

public enum ClipboardAction: Codable, Sendable {
    case copyEntry(id: UUID, plainTextOnly: Bool = false)
    case pasteEntry(id: UUID)
    case togglePin(id: UUID)
}
