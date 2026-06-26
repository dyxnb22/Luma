import Foundation

public enum SnippetsAction: Codable, Sendable {
    case copy(id: UUID)
    case paste(id: UUID)
    case create(title: String)
}
