import Foundation

public enum ClipboardAction: Codable, Sendable {
    case copyEntry(id: UUID)
}
