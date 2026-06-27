import Foundation

public enum ClipboardAction: Codable, Sendable {
    case copyEntry(id: UUID)
    case pasteEntry(id: UUID)
    case togglePin(id: UUID)
}
