import Foundation

public enum TodoAction: Codable, Sendable, Equatable {
    case create(title: String)
    case complete(id: String)
    case uncomplete(id: String)
    case grant
    case requestAccess
}
