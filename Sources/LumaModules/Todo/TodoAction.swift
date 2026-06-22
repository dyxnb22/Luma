import Foundation

public enum TodoAction: Codable, Sendable {
    case create(title: String)
    case complete(id: String)
    case grant
}
